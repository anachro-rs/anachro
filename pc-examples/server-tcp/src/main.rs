use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::thread::{sleep, spawn};
use std::time::Duration;

use anachro_server::{Broker, Uuid};

use anachro_spi::arbitrator::EncLogicHLArbitrator;
use anachro_spi_tcp::TcpSpiArbLL;
use heapless::{consts, Vec as HVec};
use postcard::to_stdvec_cobs;

use bbqueue::BBBuffer;

#[derive(Default)]
struct TcpBroker {
    broker: Broker,
    session_mgr: SessionManager,
}

// Thinks in terms of uuids
#[derive(Default)]
struct SessionManager {
    new_sessions: Vec<(Uuid, EncLogicHLArbitrator<TcpSpiArbLL, consts::U4096>)>,
    sessions: HashMap<Uuid, EncLogicHLArbitrator<TcpSpiArbLL, consts::U4096>>,
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

            // TODO: keep these around somewhere?
            let out_leak = &*Box::leak(Box::new(BBBuffer::new()));
            let inc_leak = &*Box::leak(Box::new(BBBuffer::new()));

            lock.session_mgr.new_sessions.push((
                uuid,
                EncLogicHLArbitrator::new(uuid, TcpSpiArbLL::new(stream), out_leak, inc_leak)
                    .unwrap(),
            ));

            drop(out_leak);
            drop(inc_leak);
        }
    });

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

            // TODO: This is going to be awkward without a heap/temp bbqueue
            // and multiple copies :|
            for (uuid, connect) in sessions.iter_mut() {
                if connect.poll().is_err() {
                    bad_keys.insert(*uuid);
                    continue;
                }
                let mut out_msgs: HVec<_, consts::U16> = HVec::new();
                broker.process_msg(connect, &mut out_msgs).unwrap();
                for msg in out_msgs {
                    println!("Sending: {:?}", msg);
                    if let Ok(resp) = to_stdvec_cobs(&msg.msg) {
                        responses.push((msg.dest, resp));
                    }
                }
            }

            for (dest, resp) in responses.drain(..) {
                if let Some(conn) = sessions.get_mut(&dest) {
                    let fail = conn.enqueue(&resp).is_err();

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

        sleep(Duration::from_millis(1));
    }
}
