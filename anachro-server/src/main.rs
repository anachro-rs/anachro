use postcard::to_stdvec_cobs;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use postcard::from_bytes_cobs;
use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::io::{ErrorKind, Read, Write};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

use anachro_icd::{
    arbitrator::{self, Arbitrator},
    component::{Component, ComponentInfo, Control, ControlType, PubSub, PubSubShort, PubSubType},
    PubSubPath,
};

#[derive(Default)]
struct TcpBroker {
    broker: Broker,
    session_mgr: SessionManager,
}

// Thinks in term of uuids
#[derive(Default)]
struct Broker {
    clients: Vec<Client>,
}

impl Broker {
    fn client_by_id_mut(&mut self, id: &Uuid) -> Result<&mut Client, ()> {
        self.clients
            .iter_mut()
            .find(|c| &c.id == id)
            .ok_or(())
    }

    fn process_msg(&mut self, mut req: Request) -> Result<Vec<Response>, ()> {
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

    fn process_publish(&mut self, path: &PubSubPath, payload: &[u8], source: &Uuid) -> Result<Vec<Response>, ()> {
        let useful = || self.clients.iter().filter_map(|c| {
            c.state.as_connected().ok().map(|x| (c, x))
        });

        // TODO: Make sure we're not publishing to wildcards

        // First, find the sender's path
        let source_id = useful().find(|(c, _x)| &c.id == source).ok_or(())?;
        let path = match path {
            PubSubPath::Long(lp) => *lp,
            PubSubPath::Short(sid) => {
                &source_id.1.shortcuts.iter().find(|s| &s.short == sid).ok_or(())?.long
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
                            println!("Sending 'short_{}':'{:?}' to {}", short.short, payload, client.id);
                            let msg = Arbitrator::PubSub(Ok(arbitrator::PubSubResponse::SubMsg {
                                path: PubSubPath::Short(short.short),
                                payload,
                            }));
                            let msg_bytes = to_stdvec_cobs(&msg).map_err(drop)?;
                            responses.push(Response {
                                dest: client.id.clone(),
                                msg: msg_bytes,
                            });
                            continue 'client;
                        }
                    }

                    println!("Sending '{}':'{:?}' to {}", path, payload, client.id);

                    let msg = Arbitrator::PubSub(Ok(arbitrator::PubSubResponse::SubMsg {
                        path: PubSubPath::Long(&path),
                        payload,
                    }));
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

fn matches(subscr: &str, publ: &str) -> bool {
    if subscr.is_empty() || publ.is_empty() {
        return false;
    }

    let mut s_iter = subscr.split("/");
    let mut p_iter = publ.split("/");

    loop {
        match (s_iter.next(), p_iter.next()) {
            (Some(subp), Some(pubp)) => match subp {
                "#" => return true,
                "+" => {},
                i if i == pubp => {},
                _ => return false,
            }
            (None, None) => return true,
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
                ClientState::SessionEstablished
                | ClientState::Disconnected
                | ClientState::Connected(_) => {
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

                    Some(ClientState::Connected( ConnectedState {
                        name: name.to_string(),
                        version: version.to_string(),
                        subscriptions: vec![],
                        shortcuts: vec![],
                    }))
                }
                ClientState::FatalError => None,
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

    fn process_subscribe(&mut self, path: &PubSubPath) -> Result<(), ()> {
        let state = self.state.as_connected_mut()?;

        match path {
            PubSubPath::Long(lp) => {
                state.subscriptions.push(lp.to_string());
            }
            PubSubPath::Short(sid) => {
                state.subscriptions.push(
                    state.shortcuts.iter().find(|s| &s.short == sid).ok_or(())?.long.to_string()
                );
            }
        }

        Ok(())
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
    Disconnected,
    FatalError,
}

impl ClientState {
    fn as_connected(&self) -> Result<&ConnectedState, ()> {
        match self {
            ClientState::Connected(state) => Ok(state),
            _ => Err(())
        }
    }

    fn as_connected_mut(&mut self) -> Result<&mut ConnectedState, ()> {
        match self {
            ClientState::Connected(ref mut state) => Ok(state),
            _ => Err(())
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
    short: u16
}

// Thinks in terms of uuids
#[derive(Default)]
struct SessionManager {
    new_sessions: Vec<(Uuid, Connect)>,
    sessions: HashMap<Uuid, Connect>,
}

struct Connect {
    stream: TcpStream,
    addr: SocketAddr,
    pending_data: Vec<u8>,
}

fn main() {
    let tcpb_1 = Arc::new(Mutex::new(TcpBroker::default()));
    let tcpb_2 = tcpb_1.clone();

    let hdl = spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

        while let Ok((stream, addr)) = listener.accept() {
            let uuid = Uuid::new_v4();
            stream.set_nonblocking(true).unwrap();
            println!("{:?} connected as {:?}", addr, uuid);
            let mut lock = tcpb_2.lock().unwrap();
            lock.session_mgr.new_sessions.push((
                uuid,
                Connect {
                    stream,
                    addr,
                    pending_data: vec![],
                },
            ));
        }
    });

    let mut buf = [0u8; 1024];

    loop {
        {
            let TcpBroker {
                broker,
                session_mgr:
                    SessionManager {
                        new_sessions,
                        sessions,
                    },
            } = &mut *tcpb_1.lock().unwrap();

            // Check for new connections
            for (uuid, cnct) in new_sessions.drain(..) {
                sessions.insert(uuid, cnct);
                broker.clients.push(Client {
                    id: uuid,
                    state: ClientState::SessionEstablished,
                });
            }

            let mut bad_keys = HashSet::new();
            let mut responses = vec![];

            // As a session manager, catch up with any messages
            for (key, connect) in sessions.iter_mut() {
                match connect.stream.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        connect.pending_data.extend_from_slice(&buf[..n]);
                    }
                    // Ignore empty reports
                    Ok(_) => {}
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}

                    // Evict on bad messages
                    Err(e) => {
                        println!("{:?} is bad because: {:?}. Removing", key, e);
                        bad_keys.insert(key.clone());
                    }
                }

                // Process any messages
                while let Some(p) = connect.pending_data.iter().position(|c| *c == 0x00) {
                    let mut remainder = connect.pending_data.split_off(p + 1);
                    core::mem::swap(&mut remainder, &mut connect.pending_data);
                    let payload = remainder;

                    responses.append(
                        &mut broker.process_msg(
                            Request {
                                source: key.clone(),
                                msg: payload,
                            },
                        )
                        .unwrap(),
                    );
                }
            }

            for msg in responses {
                let mut fail = false;
                if let Some(conn) = sessions.get_mut(&msg.dest) {
                    fail = conn.stream.write(&msg.msg).is_err();
                }
                if fail {
                    bad_keys.insert(msg.dest.clone());
                }
            }

            // Do evictions
            for bk in bad_keys.iter() {
                sessions.remove(bk);
            }
        }

        println!("Sleeping...");
        sleep(Duration::from_millis(1000));
    }

    hdl.join().unwrap();
}

struct Request {
    source: Uuid,
    msg: Vec<u8>,
}

struct Response {
    dest: Uuid,
    msg: Vec<u8>,
}
