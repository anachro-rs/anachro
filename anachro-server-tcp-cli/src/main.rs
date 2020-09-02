use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::io::{ErrorKind, Read, Write};
use std::thread::{sleep, spawn};
use std::time::Duration;

use anachro_server::{Broker, Request, Uuid, RESET_MESSAGE, Response};

use postcard::{from_bytes_cobs, to_stdvec_cobs};

#[derive(Default)]
struct TcpBroker {
    broker: Broker,
    session_mgr: SessionManager,
}

// Thinks in terms of uuids
#[derive(Default)]
struct SessionManager {
    new_sessions: Vec<(Uuid, Connect)>,
    sessions: HashMap<Uuid, Connect>,
}

struct Connect {
    stream: TcpStream,
    pending_data: Vec<u8>,
}

use rand::random;

fn main() {
    let broker = TcpBroker::default();

    println!("size: {}", core::mem::size_of::<Broker>());

    let tcpb_1 = Arc::new(Mutex::new(broker));
    let tcpb_2 = tcpb_1.clone();

    let _hdl = spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

        while let Ok((stream, addr)) = listener.accept() {
            let uuid_buf: u128 = random();
            let uuid = Uuid::from_bytes(uuid_buf.to_le_bytes());
            stream.set_nonblocking(true).unwrap();
            println!("{:?} connected as {:?}", addr, uuid);
            let mut lock = tcpb_2.lock().unwrap();
            lock.session_mgr.new_sessions.push((
                uuid,
                Connect {
                    stream,
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
                broker.register_client(&uuid).unwrap();
            }

            let mut bad_keys = HashSet::new();

            let mut responses = vec![];

            // As a session manager, catch up with any messages
            for (key, connect) in sessions.iter_mut() {
                // !!!!!
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
                    let mut payload = remainder;

                    println!("From {:?} got {:?}", key, payload);

                    if let Ok(msg) = from_bytes_cobs(&mut payload) {
                        let src = key.clone();
                        let req = Request { source: src, msg };

                        let resps = match broker.process_msg(&req) {
                            Ok(resps) => resps,
                            Err(e) => {
                                println!("Error: {:?}, resetting client", e);
                                let mut resp = heapless::Vec::new();
                                resp.push(Response {
                                    dest: src,
                                    msg: RESET_MESSAGE,
                                }).ok();
                                resp
                            }
                        };

                        for respmsg in resps {
                            if let Ok(resp) = to_stdvec_cobs(&respmsg.msg) {
                                responses.push((respmsg.dest, resp));
                            }
                        }
                    }
                }
            }

            for (dest, resp) in responses.drain(..) {
                if let Some(conn) = sessions.get_mut(&dest) {
                    let fail = conn.stream.write_all(&resp).is_err();

                    if fail {
                        println!("Removing {:?}", dest);
                        bad_keys.insert(dest.clone());
                    } else {
                        println!("Sent to {:?}", dest);
                    }
                }
            }

            // Do evictions
            for bk in bad_keys.iter() {
                broker.remove_client(&bk).ok();
                sessions.remove(bk);
            }
        }

        sleep(Duration::from_millis(50));
    }
}
