use anachro_icd::{
    arbitrator::Arbitrator,
    component::Component,
    PubSubPath, Version, Path,
};
use postcard::{from_bytes_cobs, to_stdvec_cobs};
use std::io::prelude::*;
use std::net::TcpStream;

use std::time::{Duration, Instant};

use anachro_client::{Client, Error, ClientIo, ClientError, pubsub_table};
use postcard;

use serde::{Serialize, Deserialize};

struct TcpAnachro {
    stream: TcpStream,
    scratch: Vec<u8>,
    current: Option<Vec<u8>>,
}

impl ClientIo for TcpAnachro {
    fn recv(&mut self) -> Result<Option<Arbitrator>, ClientError>
    {
        let mut scratch = [0u8; 1024];

        loop {
            match self.stream.read(&mut scratch) {
                Ok(n) if n > 0 => {
                    self.scratch.extend_from_slice(&scratch[..n]);

                    if let Some(p) = self.scratch.iter().position(|c| *c == 0x00) {
                        let mut remainder = self.scratch.split_off(p + 1);
                        core::mem::swap(&mut remainder, &mut self.scratch);
                        self.current = Some(remainder);

                        if let Some(ref mut payload) = self.current {
                            if let Ok(msg) = from_bytes_cobs::<Arbitrator>(payload.as_mut_slice()) {
                                println!("GIVING: {:?}", msg);
                                return Ok(Some(msg));
                            }
                        }

                        return Err(ClientError::ParsingError);
                    }
                }
                Ok(_) => return Ok(None),
                Err(_) => return Ok(None),
            }
        }


    }
    fn send(&mut self, msg: &Component) -> Result<(), ClientError> {
        println!("SENDING: {:?}", msg);
        let ser = to_stdvec_cobs(msg).map_err(|_| ClientError::ParsingError)?;
        self.stream.write_all(&ser).map_err(|_| ClientError::OutputFull)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Demo {
    foo: u32,
    bar: i16,
    baz: (u8, u8),
}

pubsub_table!{
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

    let mut cio = TcpAnachro {
        stream,
        scratch: Vec::new(),
        current: None,
    };

    // name: &str,
    // version: Version,
    // ctr_init: u16,
    // sub_paths: &'static [&'static str],
    // pub_short_paths: &'static [&'static str],
    // timeout_ticks: Option<u8>,

    let mut client = Client::new(
        "cool-board",
        Version { major: 0, minor: 4, trivial: 1, misc: 123 },
        987,
        AnachroTable::sub_paths(),
        AnachroTable::pub_paths(),
        Some(100),
    );

    while !client.is_connected() {

        match client.process_one::<_, AnachroTable>(&mut cio) {
            Ok(Some(msg)) => println!("Got: {:?}", msg),
            Ok(None) => {},
            Err(Error::ClientIoError(ClientError::NoData)) => {},
            Err(e) => println!("error: {:?}", e),
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    println!("Connected.");

    let mut ctr = 0;
    let mut last_tx = Instant::now();

    while ctr < 10 {
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

            client.publish(
                &mut cio,
                "foo/bar/baz",
                to_send,
            ).unwrap();

            println!("Sending...");
        }
        if let Ok(Some(msg)) = client.process_one::<_, AnachroTable>(&mut cio) {
            println!("{:?}", msg);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
