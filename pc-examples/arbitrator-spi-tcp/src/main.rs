#![allow(unused_imports)]

use anachro_spi::{
    self as spi,
    Error as SpiError,
    Result as SpiResult,
    arbitrator::{
        EncLogicLLArbitrator,
        EncLogicHLArbitrator,
    },
    tcp::TcpSpiArbLL,
};
use anachro_icd::Uuid;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use std::collections::{HashMap, HashSet, VecDeque};
use std::default::Default;
use std::io::{ErrorKind, Read, Write};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

use postcard::{from_bytes_cobs, to_stdvec_cobs};
use serde::{Deserialize, Serialize};

use bbqueue::{
    consts::*,
    framed::{FrameConsumer, FrameProducer, FrameGrantR, FrameGrantW},
    ArrayLength, BBBuffer, ConstBBBuffer,
};

// NOTE: For arbitrator, I will need 2xBBQueues per connection.
// this might be sort of unwieldy for something like 7-8 connections?
static BB_OUT: BBBuffer<U2048> = BBBuffer( ConstBBBuffer::new() );
static BB_INP: BBBuffer<U2048> = BBBuffer( ConstBBBuffer::new() );

fn main() {
    // FOR NOW, just accept a single connection.
    // Deal with parallel later
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    while let Ok((stream, addr)) = listener.accept() {
        let mut last_tx = Instant::now();

        stream.set_nonblocking(true).unwrap();
        println!("{:?} connected", addr);
        let mut arb = EncLogicHLArbitrator::new(
            Uuid::from_bytes([0u8; 16]), // TODO
            TcpSpiArbLL::new(stream),
            &BB_OUT,
            &BB_INP
        ).unwrap();

        while let Ok(_) = arb.poll() {
            while let Some(msg) = arb.dequeue() {
                println!("==> Got HL msg: {:?}", &msg[..]);
                msg.release();
            }

            if last_tx.elapsed() > Duration::from_secs(10) {
                println!("==> Enqueuing!");
                arb.enqueue(&[0xF0; 6]).unwrap();
                last_tx = Instant::now();
            }
        }
    }
}
