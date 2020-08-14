use std::io::prelude::*;
use std::net::TcpStream;
use postcard::{to_stdvec_cobs, from_bytes_cobs};
use anachro_icd::{
    PubSubPath,
    component::{
        Component, Control, ControlType,
        ComponentInfo, PubSub, PubSubType,
    },
    arbitrator::{
        Arbitrator,
    },
};

use std::thread::sleep;
use std::time::{Duration, Instant};

fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();

    let name = "cool-board";
    let version = "v0.1.0";

    let connect = to_stdvec_cobs(
        &Component::Control(Control {
            seq: 0x0504,
            ty: ControlType::RegisterComponent(ComponentInfo {
                name,
                version,
            })
        })
    ).unwrap();

    stream.set_nonblocking(true).unwrap();

    stream.write(&connect).unwrap();

    let path = "foo/bar/baz";
    let payload = b"henlo, welt!";

    let subscribe = to_stdvec_cobs(
        &Component::PubSub(PubSub {
            path: PubSubPath::Long(path),
            ty: PubSubType::Sub,
        })
    ).unwrap();
    stream.write(&subscribe).unwrap();

    for i in 30..40 {
        let publish = to_stdvec_cobs(
            &Component::PubSub(PubSub {
                path: PubSubPath::Long(path),
                ty: PubSubType::Pub {
                    payload,
                }
            })
        ).unwrap();
        stream.write(&publish).unwrap();
        let now = Instant::now();
        let mut scratch = [0u8; 1024];
        let mut current = vec![];
        while now.elapsed() < Duration::from_secs(2) {
            match stream.read(&mut scratch) {
                Ok(n) if n > 0 => {
                    current.extend_from_slice(&scratch[..n]);
                    while let Some(p) = current.iter().position(|c| *c == 0x00) {
                        let mut remainder = current.split_off(p + 1);
                        core::mem::swap(&mut remainder, &mut current);
                        let mut payload = remainder;
                        match from_bytes_cobs::<Arbitrator>(payload.as_mut_slice()) {
                            Ok(msg) => {
                                println!("  => Got {:?}", msg);
                            }
                            _ => {
                                println!("  => Deser Err");
                            }
                        }
                    }
                },
                _ => {
                    sleep(Duration::from_millis(50))
                }
            }
        }
    }
}
