use anachro_icd::Version;
use std::net::TcpStream;

use std::time::{Duration, Instant};

use anachro_client::{pubsub_table, Client, ClientIoError, Error};
use postcard;

use serde::{Deserialize, Serialize};

use anachro_spi::{component::EncLogicHLComponent, tcp::TcpSpiComLL};

use bbqueue::{consts::*, BBBuffer, ConstBBBuffer};

static BUF_OUT: BBBuffer<U4096> = BBBuffer(ConstBBBuffer::new());
static BUF_INP: BBBuffer<U4096> = BBBuffer(ConstBBBuffer::new());

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Demo {
    foo: u32,
    bar: i16,
    baz: (u8, u8),
}

pubsub_table! {
    AnachroTable,
    Subs => {
        Something: "foo/bar/baz" => Demo,
        Else: "bib/bim/bap" => (),
    },
    Pubs => {
        Etwas: "short/send" => (),
        Anders: "send/short" => (),
    },
}

fn main() {
    let stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    stream.set_nonblocking(true).unwrap();

    let mut cio = EncLogicHLComponent::new(TcpSpiComLL::new(stream), &BUF_OUT, &BUF_INP).unwrap();

    // name: &str,
    // version: Version,
    // ctr_init: u16,
    // sub_paths: &'static [&'static str],
    // pub_short_paths: &'static [&'static str],
    // timeout_ticks: Option<u8>,

    let mut client = Client::new(
        "cool-board",
        Version {
            major: 0,
            minor: 4,
            trivial: 1,
            misc: 123,
        },
        987,
        AnachroTable::sub_paths(),
        AnachroTable::pub_paths(),
        Some(255),
    );

    while !client.is_connected() {
        // AJM: We shouldn't have to manually poll the IO like this
        if let Err(e) = cio.poll() {
            client.reset_connection();
            continue;
        }

        match client.process_one::<_, AnachroTable>(&mut cio) {
            Ok(Some(msg)) => println!("Got: {:?}", msg),
            Ok(None) => {}
            Err(Error::ClientIoError(ClientIoError::NoData)) => {}
            Err(e) => println!("error: {:?}", e),
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    println!("Connected.");

    let mut ctr = 0;
    let mut last_tx = Instant::now();

    while ctr < 10 {
        cio.poll().unwrap();
        if last_tx.elapsed() >= Duration::from_secs(2) {
            last_tx = Instant::now();
            ctr += 1;

            let mut buf = [0u8; 1024];
            let msg = Demo {
                foo: 123,
                bar: 456,
                baz: (10, 20),
            };

            let to_send = postcard::to_slice(&msg, &mut buf).unwrap();

            client.publish(&mut cio, "foo/bar/baz", to_send).unwrap();

            println!("Sending...");
        }
        if let Ok(Some(msg)) = client.process_one::<_, AnachroTable>(&mut cio) {
            println!("{:?}", msg);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
