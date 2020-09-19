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
    let stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    stream.set_nonblocking(true).unwrap();

    println!("Component connected!");
    let mut com = TcpSpiCom::new(stream);

    while let Ok(_) = com.process() {
        sleep(Duration::from_millis(10));
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum TcpSpiMsg {
    ReadyState(bool),
    GoState(bool),
    Payload(Vec<u8>),
}

struct TcpSpiCom {
    stream: TcpStream,
    go_state: Option<bool>,
    ready_state: bool,
    outgoing_payloads: VecDeque<Vec<u8>>,
    incoming_msgs: VecDeque<Vec<u8>>,
    pending_data: Vec<u8>,
    pending_exchange: Option<PendingExchange>,
}

struct PendingExchange {
    data_out: *const u8,
    data_out_len: usize,
    data_in: *mut u8,
    data_in_max: usize,
}

impl TcpSpiCom {
    fn new(mut stream: TcpStream) -> Self {
        let init_msg = to_stdvec_cobs(
            &TcpSpiMsg::ReadyState(false)
        ).unwrap();

        // Send init message declaring GO state
        stream.write_all(&init_msg).unwrap();

        TcpSpiCom {
            stream,
            go_state: None,
            ready_state: false,
            outgoing_payloads: VecDeque::default(),
            incoming_msgs: VecDeque::default(),
            pending_data: Vec::default(),
            pending_exchange: None,
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
                    TcpSpiMsg::GoState(state) => {
                        self.go_state = Some(state);
                    }
                    TcpSpiMsg::ReadyState(_) => {
                        panic!("We're a component! No one should be sending us ready state!");
                    }
                    TcpSpiMsg::Payload(payload) => {
                        self.incoming_msgs.push_front(payload);
                    }
                }
            }
        }

        while let Some(msg) = self.outgoing_payloads.pop_back() {
            let wrapped = to_stdvec_cobs(&TcpSpiMsg::Payload(msg)).unwrap();
            self.stream.write_all(&wrapped).unwrap();
        }

        Ok(())
    }
}
