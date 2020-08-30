use anachro_icd::{
    arbitrator::Arbitrator,
    component::Component,
    PubSubPath, Version, Path,
};
use postcard::{from_bytes_cobs, to_stdvec_cobs};
use std::io::prelude::*;
use std::net::TcpStream;

use std::time::{Duration, Instant};

use anachro_client::{Client, ClientIo, ClientError, table_recv};
use postcard;
use serde::de::Deserialize;

struct TcpAnachro {
    stream: TcpStream,
    scratch: Vec<u8>,
}

impl ClientIo for TcpAnachro {
    fn recv<'a, 'b: 'a, F, E>(&'b mut self, fun: F) -> Result<(), ClientError>
    where
        F: FnOnce(&'a Arbitrator<'a>) -> Result<(), anachro_client::Error>
    {
        let mut scratch = [0u8; 1024];

        loop {
            match self.stream.read(&mut scratch) {
                Ok(n) if n > 0 => {
                    self.scratch.extend_from_slice(&scratch[..n]);

                    if let Some(p) = self.scratch.iter().position(|c| *c == 0x00) {
                        let mut remainder = self.scratch.split_off(p + 1);
                        core::mem::swap(&mut remainder, &mut self.scratch);
                        let mut payload = remainder;

                        let ret = if let Ok(msg) = from_bytes_cobs::<Arbitrator>(payload.as_mut_slice()) {
                            fun(&msg).map_err(|_| ClientError::ParsingError);
                            return Ok(())
                        } else {
                            return Err(ClientError::ParsingError);
                        };

                    }
                }
                Ok(_) => return Ok(()),
                Err(_) => return Err(ClientError::NoData),
            }
        }


    }
    fn send(&mut self, msg: &Component) -> Result<(), ClientError> {
        let ser = to_stdvec_cobs(msg).map_err(|_| ClientError::ParsingError)?;
        self.stream.write_all(&ser).map_err(|_| ClientError::OutputFull)?;
        Ok(())
    }
}



table_recv!(
    AnachroTable,
    Something: "foo/bar/baz" => (),
    Else: "bib/bim/#" => (),
);

fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    stream.set_nonblocking(true).unwrap();

    let mut cio = TcpAnachro {
        stream,
        scratch: Vec::new(),
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
        AnachroTable::paths(),
        &["short/send", "send/short"],
        Some(100),
    );

    while !client.is_connected() {
        client.process_one::<_, ()>(&mut cio);
    }

    println!("Connected.");

    // let path = Path::borrow_from_str("foo/bar/baz");
    // let payload = b"henlo, welt!";

    // let outgoing = client.subscribe(PubSubPath::Long(path.clone())).unwrap();
    // let ser = to_stdvec_cobs(&outgoing).map_err(drop).unwrap();
    // stream.write_all(&ser).map_err(drop).unwrap();

    // while !client.is_subscribe_pending() {
    //     processor(&mut stream, &mut client, &mut current).unwrap();
    // }

    // println!("Subscribed.");

    // let mut ctr = 0;
    // let mut last_tx = Instant::now();

    // while ctr < 10 {
    //     if last_tx.elapsed() >= Duration::from_secs(2) {
    //         last_tx = Instant::now();
    //         ctr += 1;

    //         let msg = client.publish(PubSubPath::Long(path.clone()), payload).unwrap();

    //         println!("Sending...");

    //         let ser = to_stdvec_cobs(&msg).map_err(drop).unwrap();
    //         stream.write_all(&ser).map_err(drop).unwrap();
    //     }

    //     processor(&mut stream, &mut client, &mut current).unwrap();
    // }
}
