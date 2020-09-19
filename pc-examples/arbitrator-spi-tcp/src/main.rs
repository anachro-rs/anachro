#![allow(unused_imports)]

use anachro_spi as spi;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use std::collections::{HashMap, HashSet, VecDeque};
use std::default::Default;
use std::io::{ErrorKind, Read, Write};
use std::thread::{sleep, spawn};
use std::time::Duration;

use serde::{Serialize, Deserialize};
use postcard::{from_bytes_cobs, to_stdvec_cobs};

fn main() {
    // FOR NOW, just accept a single connection.
    // Deal with parallel later
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    while let Ok((stream, addr)) = listener.accept() {
        stream.set_nonblocking(true).unwrap();
        println!("{:?} connected", addr);
        let mut arb = TcpSpiArb::new(stream);

        while let Ok(_) = arb.process() {
            sleep(Duration::from_millis(10));
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum TcpSpiMsg {
    ReadyState(bool),
    GoState(bool),
    Payload(Vec<u8>),
}

struct TcpSpiArb {
    stream: TcpStream,
    go_state: bool,
    ready_state: Option<bool>,
    outgoing_msgs: VecDeque<Vec<u8>>,
    incoming_msgs: VecDeque<Vec<u8>>,
    pending_data: Vec<u8>,
}

impl TcpSpiArb {
    fn new(mut stream: TcpStream) -> Self {
        let init_msg = to_stdvec_cobs(
            &TcpSpiMsg::GoState(false)
        ).unwrap();

        // Send init message declaring GO state
        stream.write_all(&init_msg).unwrap();

        TcpSpiArb {
            stream,
            go_state: false,
            ready_state: None,
            outgoing_msgs: VecDeque::default(),
            incoming_msgs: VecDeque::default(),
            pending_data: Vec::default(),
        }
    }

    fn process(&mut self) -> Result<(), ()> {
        let mut buf = [0u8; 1024];

        // Receive incoming messages
        loop {
            match self.stream.read(&mut buf) {
                Ok(n) if n > 0 => {
                    self.pending_data.extend_from_slice(&buf[..n]);
                }
                Ok(_) => {
                    break;
                },
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    break;
                },
                Err(e) => {
                    eprintln!("TCP Error: {:?}", e);
                    panic!()
                }
            }
        }


        // Process any messages
        while let Some(p) = self.pending_data.iter().position(|c| *c == 0x00) {
            let mut remainder = self.pending_data.split_off(p + 1);
            core::mem::swap(&mut remainder, &mut self.pending_data);
            let mut payload = remainder;

            println!("TCP: got {:?}", payload);

            if let Ok(msg) = from_bytes_cobs::<TcpSpiMsg>(&mut payload) {
                match msg {
                    TcpSpiMsg::ReadyState(state) => {
                        self.ready_state = Some(state);
                    }
                    TcpSpiMsg::GoState(_) => {
                        panic!("We're an arbitrator! No one should tell us Go state!");
                    }
                    TcpSpiMsg::Payload(payload) => {
                        self.incoming_msgs.push_front(payload);
                    }
                }
            }
        }

        while let Some(msg) = self.outgoing_msgs.pop_back() {
            self.stream.write_all(&msg).unwrap();
        }

        Ok(())
    }
}
