use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::io::{ErrorKind, Read, Write};
use std::thread::{sleep, spawn};
use std::time::Duration;

use anachro_server::{Broker, Request, Response, Uuid, RESET_MESSAGE, ServerIoIn, ServerIoOut, ServerIoError};

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
    current_data: Vec<u8>,
    key: Uuid,
}

#[derive(Default)]
struct ConnectOut<'a> {
    data: Vec<Response<'a>>,
}

impl ServerIoIn for Connect {
    fn recv<'a, 'b: 'a>(&'b mut self) -> Result<Option<Request<'b>>, ServerIoError> {
        let mut buf = [0u8; 1024];
        match self.stream.read(&mut buf) {
            Ok(n) if n > 0 => {
                self.pending_data.extend_from_slice(&buf[..n]);
            }
            // Ignore empty reports
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::WouldBlock => {}

            // Evict on bad messages
            Err(e) => {
                println!("bad because: {:?}. Removing", e);

                return Err(ServerIoError::ToDo);
                // bad_keys.insert(key.clone());
            }
        }

        if let Some(p) = self.pending_data.iter().position(|c| *c == 0x00) {
            let mut remainder = self.pending_data.split_off(p + 1);
            core::mem::swap(&mut remainder, &mut self.pending_data);
            self.current_data = remainder;

            // println!("From {:?} got {:?}", key, payload);

            if let Ok(msg) = from_bytes_cobs(&mut self.current_data) {
                let src = self.key.clone();
                let req = Request { source: src, msg };
                return Ok(Some(req));
            }
        }

        Ok(None)
    }
}

impl<'resp> ServerIoOut<'resp> for ConnectOut<'resp> {
    fn push_response(&mut self, resp: Response<'resp>) -> Result<(), ServerIoError> {
        self.data.push(resp);
        Ok(())
    }
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
                    key: uuid,
                    stream,
                    pending_data: vec![],
                    current_data: vec![],
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
            for (key, connect_in) in sessions.iter_mut() {
                // !!!!!
                match connect_in.stream.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        connect_in.pending_data.extend_from_slice(&buf[..n]);
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
                let mut connect_out = ConnectOut { data: vec![] };
                let mut resps = match broker.process_msg(connect_in, &mut connect_out) {
                    Ok(()) => {
                        connect_out.data
                    },
                    Err(e) => {
                        println!("Error: {:?}, resetting client", e);
                        vec![Response {
                            dest: *key,
                            msg: RESET_MESSAGE,
                        }]
                    }
                };

                for respmsg in resps.drain(..) {
                    if let Ok(resp) = to_stdvec_cobs(&respmsg.msg) {
                        responses.push((respmsg.dest, resp));
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
