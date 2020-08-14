use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::io::{ErrorKind, Read, Write};
use std::thread::{sleep, spawn};
use std::time::Duration;

use anachro_server::{Broker, Request};

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

fn main() {
    let tcpb_1 = Arc::new(Mutex::new(TcpBroker::default()));
    let tcpb_2 = tcpb_1.clone();

    let _hdl = spawn(move || {
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
}
