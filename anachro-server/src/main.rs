use std::net::{TcpListener, TcpStream, SocketAddr};
use uuid::Uuid;
use std::sync::{Arc, Mutex};

use std::collections::HashMap;
use std::time::{Instant, Duration};
use std::thread::{spawn, sleep};
use std::io::{Read, ErrorKind};
use postcard::from_bytes_cobs;

use anachro_icd::{
    component::{
        Component,
    },
    arbitrator::{
        Arbitrator,
    },
};

struct Connect {
    stream: TcpStream,
    addr: SocketAddr,
    current: Vec<u8>,
    has_connected: bool,
    shorts: HashMap<String, u16>,
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
    subscribers: Vec<Uuid>,
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
            lock.connects.insert(uuid, Connect {
                stream,
                addr,
                current: vec![],
                has_connected: false,
                shorts: HashMap::new(),
            });
        }
    });

    loop {
        let mut buf = [0u8; 1024];
        {
            let mut lock = ctx_1.lock().unwrap();
            let mut bad_keys = vec![];
            let mut responses = vec![];

            let Context { ref mut connects, ref mut nodes } = *lock;

            // Check each connection for new data/errors
            for (key, connect) in connects.iter_mut() {
                match connect.stream.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        connect.current.extend_from_slice(&buf[..n]);
                    }
                    // Ignore empty reports
                    Ok(_) => {},
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {},

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

                    responses.append(&mut process_msg(
                        Request {
                            source: key.clone(),
                            msg: payload,
                        },
                        connect,
                        nodes,
                    ).unwrap());
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

fn process_msg(mut req: Request, connect: &mut Connect, nodes: &mut TreeNode) -> Result<Vec<Response>, ()> {
    match from_bytes_cobs::<Component>(&mut req.msg) {
        Ok(msg) => {
            println!("{:?} says {:?}", req.source, msg);
        },
        Err(e) => println!("{:?} parse error: {:?}", req.source, e),
    }
    Ok(vec![])
}
