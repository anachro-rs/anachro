use postcard::to_stdvec_cobs;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use postcard::from_bytes_cobs;
use std::collections::{HashMap, HashSet};
use std::io::{ErrorKind, Read, Write};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

use anachro_icd::{
    arbitrator::{self, Arbitrator},
    component::{Component, ComponentInfo, Control, ControlType, PubSub, PubSubShort, PubSubType},
    PubSubPath,
};

struct Connect {
    stream: TcpStream,
    addr: SocketAddr,
    current: Vec<u8>,
    shorts: HashMap<u16, String>,

    // TODO: Group these
    has_connected: bool,
    name: Option<String>,
    version: Option<String>,
}

struct TimedData {
    data: Vec<u8>,
    expires: Instant,
}

#[derive(Default)]
struct TreeNode {
    this: Option<TimedData>,
    path: String,
    children: HashMap<String, TreeNode>,

    // TODO: This only works if subscribe comes after
    // first publish. We need a way to retroactively
    // apply subscriptions to new topics
    subscribers: HashSet<Uuid>,
}

impl TreeNode {
    fn insert(&mut self, path: &str, data: &[u8], expires: u16) -> Result<Vec<Uuid>, ()> {
        let segs = path.split('/');
        let mut node = self;

        for seg in segs {
            let path = node.path.clone() + "/" + seg;
            node = node.children.entry(seg.to_owned()).or_insert_with(|| {
                TreeNode {
                    this: None,
                    path,
                    children: HashMap::new(),
                    subscribers: HashSet::new(), // TODO: Definitely wrong
                }
            });
        }

        node.this = Some(TimedData {
            data: data.to_vec(),
            expires: Instant::now() + Duration::from_secs(expires.into()),
        });

        Ok(node.subscribers.iter().cloned().collect::<Vec<_>>())
    }

    fn subscribe(&mut self, path: &str, uuid: Uuid) -> Result<(), ()> {
        let segs = path.split('/');
        let mut node = self;

        for seg in segs {
            let path = node.path.clone() + "/" + seg;
            node = node.children.entry(seg.to_owned()).or_insert_with(|| {
                TreeNode {
                    this: None,
                    path,
                    children: HashMap::new(),
                    subscribers: HashSet::new(), // TODO: Definitely wrong
                }
            });
        }

        node.subscribers.insert(uuid);
        Ok(())
    }
}

type Connects = HashMap<Uuid, Connect>;

#[derive(Default)]
struct Context {
    connects: Connects,

    // TODO: Top level should be just UUIDs?
    nodes: TreeNode,
}

struct Request {
    source: Uuid,
    msg: Vec<u8>,
}

struct Response {
    dest: Uuid,
    msg: Vec<u8>,
}

fn main() {
    let ctx_1 = Arc::new(Mutex::new(Context::default()));
    let ctx_2 = ctx_1.clone();

    let hdl = spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

        while let Ok((stream, addr)) = listener.accept() {
            let uuid = Uuid::new_v4();
            stream.set_nonblocking(true).unwrap();
            println!("{:?} connected as {:?}", addr, uuid);
            let mut lock = ctx_2.lock().unwrap();
            lock.connects.insert(
                uuid,
                Connect {
                    stream,
                    addr,
                    current: vec![],
                    has_connected: false,
                    shorts: HashMap::new(),
                    name: None,
                    version: None,
                },
            );
        }
    });

    loop {
        let mut buf = [0u8; 1024];
        {
            let mut lock = ctx_1.lock().unwrap();
            let mut bad_keys = vec![];
            let mut responses = vec![];

            let Context {
                ref mut connects,
                ref mut nodes,
            } = *lock;

            // Check each connection for new data/errors
            for (key, connect) in connects.iter_mut() {
                match connect.stream.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        connect.current.extend_from_slice(&buf[..n]);
                    }
                    // Ignore empty reports
                    Ok(_) => {}
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}

                    // Evict on bad messages
                    Err(e) => {
                        println!("{:?} is bad because: {:?}. Removing", key, e);
                        bad_keys.push(key.clone());
                    }
                }

                // Process any messages
                while let Some(p) = connect.current.iter().position(|c| *c == 0x00) {
                    let mut remainder = connect.current.split_off(p + 1);
                    core::mem::swap(&mut remainder, &mut connect.current);
                    let payload = remainder;

                    responses.append(
                        &mut process_msg(
                            Request {
                                source: key.clone(),
                                msg: payload,
                            },
                            connect,
                            nodes,
                        )
                        .unwrap(),
                    );
                }


            }

            for msg in responses {
                let mut fail = false;
                if let Some(conn) = connects.get_mut(&msg.dest) {
                    fail = conn.stream.write(&msg.msg).is_err();
                }
                if fail {
                    // TODO: Also need to remove from the subscriber list!
                    connects.remove(&msg.dest);
                }
            }

            // Do evictions
            for bk in bad_keys.iter() {
                lock.connects.remove(bk);
            }
        }

        println!("Sleeping...");
        sleep(Duration::from_millis(1000));
    }

    hdl.join();
}

fn process_msg(
    mut req: Request,
    connect: &mut Connect,
    nodes: &mut TreeNode,
) -> Result<Vec<Response>, ()> {
    let mut responses = vec![];

    match from_bytes_cobs::<Component>(&mut req.msg) {
        Ok(msg) => {
            match msg {
                Component::Control(Control { seq, ty }) => match ty {
                    ControlType::RegisterComponent(ComponentInfo { name, version }) => {
                        if !connect.has_connected {
                            println!("{:?} registered as {}, {}", req.source, name, version);
                            connect.name = Some(name.to_owned());
                            connect.version = Some(version.to_owned());
                            let resp = Arbitrator::Control(arbitrator::Control {
                                seq,
                                response: Ok(arbitrator::ControlResponse::ComponentRegistration(
                                    req.source.clone(),
                                )),
                            });
                            let resp_bytes = to_stdvec_cobs(&resp).unwrap();
                            responses.push(Response {
                                dest: req.source.clone(),
                                msg: resp_bytes,
                            });
                        }
                        connect.has_connected = true;
                    }
                    ControlType::RegisterPubSubShortId(PubSubShort {
                        long_name,
                        short_id,
                    }) => {
                        if !connect.has_connected {
                            return Err(());
                        }
                        println!("{:?} aliased '{}' to {}", req.source, long_name, short_id);
                        connect.shorts.insert(short_id, long_name.to_owned());
                    }
                },
                Component::PubSub(PubSub { path, ty }) if connect.has_connected => {
                    match ty {
                        PubSubType::Pub {
                            payload,
                            validity_sec_max,
                        } => {
                            let path = match path {
                                PubSubPath::Long(lng) => lng,
                                PubSubPath::Short(shr) => connect.shorts.get(&shr).ok_or_else(|| ())?,
                            }.to_owned();

                            let to_notify = nodes.insert(&path, payload, validity_sec_max)?;

                            for id in to_notify {
                                // TODO: We need to look up the short path for each subscriber!
                                // For this we need the whole connects list
                                let msg = Arbitrator::PubSub(Ok(arbitrator::PubSubResponse::SubMsg {
                                    path: PubSubPath::Long(&path),
                                    payload,
                                }));
                                let msg_bytes = to_stdvec_cobs(&msg).map_err(drop)?;
                                responses.push(Response {
                                    dest: id.clone(),
                                    msg: msg_bytes,
                                });
                            }

                            // TODO: Notify all subscribers
                        }
                        // TODO: Periodic option for sub? min/max rate?
                        PubSubType::Sub => {
                            let path = match path {
                                PubSubPath::Long(lng) => lng,
                                PubSubPath::Short(shr) => connect.shorts.get(&shr).ok_or_else(|| ())?,
                            }.to_owned();

                            println!("{} subscribed to '{}'", connect.name.as_ref().unwrap(), &path);
                            nodes.subscribe(&path, req.source.clone())?;
                        }
                        PubSubType::Unsub => {}
                        PubSubType::Get => {}
                    }
                }
                _ => {}
            }
        }
        Err(e) => println!("{:?} parse error: {:?}", req.source, e),
    }
    Ok(responses)
}
