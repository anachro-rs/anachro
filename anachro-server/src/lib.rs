#![no_std]

use core::default::Default;

use anachro_icd::{
    arbitrator::{self, Arbitrator, SubMsg},
    component::{Component, ComponentInfo, Control, ControlType, PubSub, PubSubShort, PubSubType},
};

pub use anachro_icd::{PubSubPath, Version, Name, Path, Uuid};

use heapless::{consts, Vec};

type ClientStore = Vec<Client, consts::U8>;

// Thinks in term of uuids
#[derive(Default)]
pub struct Broker {
    clients: ClientStore,
}

impl Broker {
    fn client_by_id_mut(&mut self, id: &Uuid) -> Result<&mut Client, ()> {
        self.clients.iter_mut().find(|c| &c.id == id).ok_or(())
    }

    pub fn register_client(&mut self, id: &Uuid) -> Result<(), ()> {
        if self.clients.iter_mut().find(|c| &c.id == id).is_none() {
            self.clients
                .push(Client {
                    id: id.clone(),
                    state: ClientState::SessionEstablished,
                })
                .map_err(drop)?;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn remove_client(&mut self, id: &Uuid) -> Result<(), ()> {
        let pos = self.clients.iter().position(|c| &c.id == id).ok_or(())?;
        self.clients.swap_remove(pos);
        Ok(())
    }

    pub fn process_one<'a, 'b: 'a, S: ServerIo>(
        &'b mut self,
        sio: &mut S,
    ) -> Result<(), ()> {
        let req = sio.recv().map_err(drop)?;
        let req = match req {
            Some(msg) => msg,
            None => return Ok(())
        };

        let mut responses = Vec::new();

        match &req.msg {
            Component::Control(ctrl) => {
                let client = self.client_by_id_mut(&req.source)?;

                if let Some(msg) = client.process_control(&ctrl)? {
                    responses.push(msg).map_err(drop)?;
                }
            }
            Component::PubSub(PubSub { ref path, ref ty }) => match ty {
                PubSubType::Pub { ref payload } => {
                    responses = self.process_publish(path, payload, &req.source)?;
                }
                PubSubType::Sub => {
                    let client = self.client_by_id_mut(&req.source)?;
                    responses.push(client.process_subscribe(&path)?).map_err(drop)?;
                }
                PubSubType::Unsub => {
                    let client = self.client_by_id_mut(&req.source)?;
                    client.process_unsub(&path)?;
                    todo!()
                }
            },
        }

        for resp in responses {
            sio.send(&resp).ok();
        }

        Ok(())
    }

    fn process_publish<'b: 'a, 'a>(
        &'b mut self,
        path: &'a PubSubPath,
        payload: &'a [u8],
        source: &'a Uuid,
    ) -> Result<Vec<Response<'a>, consts::U8>, ()> {
        // TODO: Make sure we're not publishing to wildcards

        // First, find the sender's path
        let source_id = self
            .clients
            .iter()
            .filter_map(|c| c.state.as_connected().ok().map(|x| (c, x)))
            .find(|(c, _x)| &c.id == source)
            .ok_or(())?;
        let path = match path {
            PubSubPath::Long(lp) => lp.as_str(),
            PubSubPath::Short(sid) => &source_id
                .1
                .shortcuts
                .iter()
                .find(|s| &s.short == sid)
                .ok_or(())?
                .long
                .as_str(),
        };

        // println!("{:?} said '{:?}' to {}", source, payload, path);

        // Then, find all applicable destinations, max of 1 per destination
        let mut responses = Vec::new();
        'client: for (client, state) in self
            .clients
            .iter()
            .filter_map(|c| c.state.as_connected().ok().map(|x| (c, x)))
        {
            if &client.id == source {
                // Don't send messages back to the sender
                continue;
            }

            for subt in state.subscriptions.iter() {
                if anachro_icd::matches(subt.as_str(), path) {
                    // Does the destination have a shortcut for this?
                    for short in state.shortcuts.iter() {
                        // NOTE: we use path, NOT subt, as it may contain wildcards
                        if path == short.long.as_str() {
                            // println!(
                            //     "Sending 'short_{}':'{:?}' to {:?}",
                            //     short.short, payload, client.id
                            // );
                            let msg = Arbitrator::PubSub(Ok(arbitrator::PubSubResponse::SubMsg(
                                SubMsg {
                                    path: PubSubPath::Short(short.short),
                                    payload,
                                },
                            )));
                            responses
                                .push(Response {
                                    dest: client.id.clone(),
                                    msg,
                                })
                                .map_err(drop)?;
                            continue 'client;
                        }
                    }

                    // println!("Sending '{}':'{:?}' to {:?}", path, payload, client.id);

                    let msg = Arbitrator::PubSub(Ok(arbitrator::PubSubResponse::SubMsg(SubMsg {
                        path: PubSubPath::Long(Path::borrow_from_str(path)),
                        payload,
                    })));
                    responses
                        .push(Response {
                            dest: client.id.clone(),
                            msg,
                        })
                        .map_err(drop)?;
                    continue 'client;
                }
            }
        }

        Ok(responses)
    }
}

struct Client {
    id: Uuid,
    state: ClientState,
}

impl Client {
    fn process_control(&mut self, ctrl: &Control) -> Result<Option<Response>, ()> {
        let mut response = None;

        let next = match &ctrl.ty {
            ControlType::RegisterComponent(ComponentInfo { name, version }) => match &self.state {
                ClientState::SessionEstablished | ClientState::Connected(_) => {
                    // println!("{:?} registered as {}, {:?}", self.id, name.as_str(), version);

                    let resp = Arbitrator::Control(arbitrator::Control {
                        seq: ctrl.seq,
                        response: Ok(arbitrator::ControlResponse::ComponentRegistration(
                            self.id.clone(),
                        )),
                    });

                    response = Some(Response {
                        dest: self.id.clone(),
                        msg: resp,
                    });

                    Some(ClientState::Connected(ConnectedState {
                        name: name.try_to_owned()?,
                        version: *version,
                        subscriptions: Vec::new(),
                        shortcuts: Vec::new(),
                    }))
                }
            },
            ControlType::RegisterPubSubShortId(PubSubShort {
                long_name,
                short_id,
            }) => {

                let state = self.state.as_connected_mut()?;

                if long_name.contains('#') || long_name.contains('+') {
                    // TODO: How to handle wildcards + short names?
                    let resp = Arbitrator::Control(arbitrator::Control {
                        seq: ctrl.seq,
                        response: Err(arbitrator::ControlError::NoWildcardsInShorts),
                    });

                    response = Some(Response {
                        dest: self.id.clone(),
                        msg: resp,
                    });

                } else {
                    if state.shortcuts.iter().find(|sc| {
                        (sc.long.as_str() == *long_name) && (sc.short == *short_id)
                    }).is_none() {
                        state
                            .shortcuts
                            .push(Shortcut {
                                long: Path::try_from_str(long_name).unwrap(),
                                short: *short_id,
                            })
                            .map_err(drop)?;
                    }

                    let resp = Arbitrator::Control(arbitrator::Control {
                        seq: ctrl.seq,
                        response: Ok(arbitrator::ControlResponse::PubSubShortRegistration(*short_id)),
                    });

                    response = Some(Response {
                        dest: self.id.clone(),
                        msg: resp,
                    });

                }

                // println!("{:?} aliased '{}' to {}", self.id, long_name, short_id);

                // TODO: Dupe check?


                None
            }
        };

        if let Some(next) = next {
            self.state = next;
        }

        Ok(response)
    }

    fn process_subscribe<'a>(&mut self, path: &'a PubSubPath) -> Result<Response<'a>, ()> {
        let state = self.state.as_connected_mut()?;

        // Determine canonical path
        let path_str = match path {
            PubSubPath::Long(lp) => lp.as_str(),
            PubSubPath::Short(sid) => state
                .shortcuts
                .iter()
                .find(|s| &s.short == sid)
                .ok_or(())?
                .long
                .as_str(),
        };

        // Only push if not a dupe
        if state
            .subscriptions
            .iter()
            .find(|s| s.as_str() == path_str)
            .is_none()
        {
            state
                .subscriptions
                .push(Path::try_from_str(path_str).unwrap())
                .map_err(drop)?;
        }

        let resp = Arbitrator::PubSub(Ok(arbitrator::PubSubResponse::SubAck {
            path: path.clone(),
        }));

        Ok(Response {
            dest: self.id.clone(),
            msg: resp,
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
    name: Name<'static>,
    version: Version,
    subscriptions: Vec<Path<'static>, consts::U8>,
    shortcuts: Vec<Shortcut, consts::U8>,
}

#[derive(Debug)]
struct Shortcut {
    long: Path<'static>,
    short: u16,
}

// TODO: These two probably shouldn't exist here, because
// it means we constrain the serialization method. In the
// future we should probably just have msg be Arbitrator/
// Component

pub struct Request<'a> {
    pub source: Uuid,
    pub msg: Component<'a>,
}

pub struct Response<'a> {
    pub dest: Uuid,
    pub msg: Arbitrator<'a>,
}

#[derive(Debug, PartialEq)]
pub enum ServerError {
    ParsingError,
    NoData,
    OutputFull,
}

pub trait ServerIo {
    fn recv(&mut self) -> Result<Option<Request>, ServerError>;
    fn send(&mut self, resp: &Response) -> Result<(), ServerError>;
}
