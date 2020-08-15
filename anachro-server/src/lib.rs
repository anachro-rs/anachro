use core::default::Default;

use postcard::{from_bytes_cobs, to_stdvec_cobs};
use uuid::Uuid;

use anachro_icd::{
    arbitrator::{self, Arbitrator, SubMsg},
    component::{Component, ComponentInfo, Control, ControlType, PubSub, PubSubShort, PubSubType},
    PubSubPath,
};

// Thinks in term of uuids
#[derive(Default)]
pub struct Broker {
    clients: Vec<Client>,
}

impl Broker {
    fn client_by_id_mut(&mut self, id: &Uuid) -> Result<&mut Client, ()> {
        self.clients.iter_mut().find(|c| &c.id == id).ok_or(())
    }

    pub fn register_client(&mut self, id: &Uuid) -> Result<(), ()> {
        if self.clients.iter_mut().find(|c| &c.id == id).is_none() {
            self.clients.push(Client {
                id: id.clone(),
                state: ClientState::SessionEstablished,
            });
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn remove_client(&mut self, id: &Uuid) -> Result<(), ()> {
        let pos = self.clients.iter().position(|c| &c.id == id).ok_or(())?;
        self.clients.remove(pos);
        Ok(())
    }

    pub fn process_msg(&mut self, mut req: Request) -> Result<Vec<Response>, ()> {
        let mut responses = vec![];

        match from_bytes_cobs::<Component>(&mut req.msg) {
            Ok(msg) => match msg {
                Component::Control(mut ctrl) => {
                    let client = self.client_by_id_mut(&req.source)?;

                    client.process_control(&mut ctrl)?.map(|msg| {
                        responses.push(msg);
                    });
                }
                Component::PubSub(PubSub { path, ty }) => match ty {
                    PubSubType::Pub { payload } => {
                        responses.append(&mut self.process_publish(&path, payload, &req.source)?);
                    }
                    PubSubType::Sub => {
                        let client = self.client_by_id_mut(&req.source)?;
                        client.process_subscribe(&path)?;
                    }
                    PubSubType::Unsub => {
                        let client = self.client_by_id_mut(&req.source)?;
                        client.process_unsub(&path)?;
                    }
                },
            },
            Err(e) => println!("{:?} parse error: {:?}", req.source, e),
        }

        Ok(responses)
    }

    fn process_publish(
        &mut self,
        path: &PubSubPath,
        payload: &[u8],
        source: &Uuid,
    ) -> Result<Vec<Response>, ()> {
        let useful = || {
            self.clients
                .iter()
                .filter_map(|c| c.state.as_connected().ok().map(|x| (c, x)))
        };

        // TODO: Make sure we're not publishing to wildcards

        // First, find the sender's path
        let source_id = useful().find(|(c, _x)| &c.id == source).ok_or(())?;
        let path = match path {
            PubSubPath::Long(lp) => *lp,
            PubSubPath::Short(sid) => {
                &source_id
                    .1
                    .shortcuts
                    .iter()
                    .find(|s| &s.short == sid)
                    .ok_or(())?
                    .long
            }
        };

        println!("{} said '{:?}' to {}", source, payload, path);

        // Then, find all applicable destinations, max of 1 per destination
        let mut responses = vec![];
        'client: for (client, state) in useful() {
            if &client.id == source {
                // Don't send messages back to the sender
                continue;
            }

            for subt in state.subscriptions.iter() {
                if matches(subt, path) {
                    // Does the destination have a shortcut for this?
                    for short in state.shortcuts.iter() {
                        // NOTE: we use path, NOT subt, as it may contain wildcards
                        if path == &short.long {
                            println!(
                                "Sending 'short_{}':'{:?}' to {}",
                                short.short, payload, client.id
                            );
                            let msg = Arbitrator::PubSub(Ok(arbitrator::PubSubResponse::SubMsg(
                                SubMsg {
                                    path: PubSubPath::Short(short.short),
                                    payload,
                                },
                            )));
                            let msg_bytes = to_stdvec_cobs(&msg).map_err(drop)?;
                            responses.push(Response {
                                dest: client.id.clone(),
                                msg: msg_bytes,
                            });
                            continue 'client;
                        }
                    }

                    println!("Sending '{}':'{:?}' to {}", path, payload, client.id);

                    let msg = Arbitrator::PubSub(Ok(arbitrator::PubSubResponse::SubMsg(SubMsg {
                        path: PubSubPath::Long(&path),
                        payload,
                    })));
                    let msg_bytes = to_stdvec_cobs(&msg).map_err(drop)?;
                    responses.push(Response {
                        dest: client.id.clone(),
                        msg: msg_bytes,
                    });
                    continue 'client;
                }
            }
        }

        Ok(responses)
    }
}

/// ## Examples
///
/// ```
/// # use anachro_server::matches;
/// #
/// assert!(matches(
///  "/+/temperature/#",
///  "/dev_1/temperature/front",
/// ));
/// ```
pub fn matches(subscr: &str, publ: &str) -> bool {
    if subscr.is_empty() || publ.is_empty() {
        return false;
    }

    let mut s_iter = subscr.split("/");
    let mut p_iter = publ.split("/");

    loop {
        match (s_iter.next(), p_iter.next()) {
            (Some("+"), Some(_)) => continue,
            (Some(lhs), Some(rhs)) if lhs == rhs => continue,
            (Some("#"), _) | (None, None) => return true,
            _ => return false,
        }
    }
}

struct Client {
    id: Uuid,
    state: ClientState,
}

impl Client {
    fn process_control(&mut self, ctrl: &mut Control) -> Result<Option<Response>, ()> {
        let mut response = None;

        let next = match ctrl.ty {
            ControlType::RegisterComponent(ComponentInfo { name, version }) => match &self.state {
                ClientState::SessionEstablished | ClientState::Connected(_) => {
                    println!("{:?} registered as {}, {}", self.id, name, version);

                    let resp = Arbitrator::Control(arbitrator::Control {
                        seq: ctrl.seq,
                        response: Ok(arbitrator::ControlResponse::ComponentRegistration(
                            self.id.clone(),
                        )),
                    });

                    let resp_bytes = to_stdvec_cobs(&resp).unwrap();
                    response = Some(Response {
                        dest: self.id.clone(),
                        msg: resp_bytes,
                    });

                    Some(ClientState::Connected(ConnectedState {
                        name: name.to_string(),
                        version: version.to_string(),
                        subscriptions: vec![],
                        shortcuts: vec![],
                    }))
                }
            },
            ControlType::RegisterPubSubShortId(PubSubShort {
                long_name,
                short_id,
            }) => {
                if long_name.contains('#') || long_name.contains('+') {
                    // TODO: How to handle wildcards + short names?
                    return Err(());
                }
                let state = self.state.as_connected_mut()?;

                println!("{:?} aliased '{}' to {}", self.id, long_name, short_id);

                // TODO: Dupe check?
                state.shortcuts.push(Shortcut {
                    long: long_name.to_owned(),
                    short: short_id,
                });
                None
            }
        };

        if let Some(next) = next {
            self.state = next;
        }

        Ok(response)
    }

    fn process_subscribe(&mut self, path: &PubSubPath) -> Result<Response, ()> {
        let state = self.state.as_connected_mut()?;

        // Determine canonical path
        let path_str = match path {
            PubSubPath::Long(lp) => lp.to_string(),
            PubSubPath::Short(sid) => state
                .shortcuts
                .iter()
                .find(|s| &s.short == sid)
                .ok_or(())?
                .long
                .to_string(),
        };

        // Only push if not a dupe
        if state
            .subscriptions
            .iter()
            .find(|s| s == &&path_str)
            .is_none()
        {
            state.subscriptions.push(path_str)
        }

        let resp = Arbitrator::PubSub(Ok(arbitrator::PubSubResponse::SubAck {
            path: path.clone(),
        }));

        let resp_bytes = to_stdvec_cobs(&resp).map_err(drop)?;

        Ok(Response {
            dest: self.id.clone(),
            msg: resp_bytes,
        })
    }

    fn process_unsub(&mut self, _path: &PubSubPath) -> Result<(), ()> {
        let _state = self.state.as_connected_mut()?;

        todo!()
    }
}

#[derive(Debug)]
enum ClientState {
    SessionEstablished,
    Connected(ConnectedState),
}

impl ClientState {
    fn as_connected(&self) -> Result<&ConnectedState, ()> {
        match self {
            ClientState::Connected(state) => Ok(state),
            _ => Err(()),
        }
    }

    fn as_connected_mut(&mut self) -> Result<&mut ConnectedState, ()> {
        match self {
            ClientState::Connected(ref mut state) => Ok(state),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
struct ConnectedState {
    name: String,
    version: String,
    subscriptions: Vec<String>,
    shortcuts: Vec<Shortcut>,
}

#[derive(Debug)]
struct Shortcut {
    long: String,
    short: u16,
}

pub struct Request {
    pub source: Uuid,
    pub msg: Vec<u8>,
}

pub struct Response {
    pub dest: Uuid,
    pub msg: Vec<u8>,
}
