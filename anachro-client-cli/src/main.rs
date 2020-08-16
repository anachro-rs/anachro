use anachro_icd::{
    arbitrator::Arbitrator,
    PubSubPath, Version, Path,
};
use postcard::{from_bytes_cobs, to_stdvec_cobs};
use std::io::prelude::*;
use std::net::TcpStream;

use std::time::{Duration, Instant};

use anachro_client::Client;

fn processor(stream: &mut TcpStream, client: &mut Client, current: &mut Vec<u8>) -> Result<(), ()> {
    let mut scratch = [0u8; 1024];

    loop {
        match stream.read(&mut scratch) {
            Ok(n) if n > 0 => {
                current.extend_from_slice(&scratch[..n]);

                while let Some(p) = current.iter().position(|c| *c == 0x00) {
                    let mut remainder = current.split_off(p + 1);
                    core::mem::swap(&mut remainder, current);
                    let mut payload = remainder;

                    match from_bytes_cobs::<Arbitrator>(payload.as_mut_slice()) {
                        Ok(msg) => {
                            println!("got {:?}", msg);
                            let smsg = Some(msg);
                            let x = client.process(&smsg)?;

                            if let Some(bmsg) = x.broker_response {
                                let ser = to_stdvec_cobs(&bmsg).map_err(drop)?;
                                stream.write_all(&ser).map_err(drop)?;
                            }

                            if let Some(cmsg) = x.client_response {
                                println!("'{:?}': '{:?}'", cmsg.path, cmsg.payload);
                            }
                        }
                        _ => {
                            println!("  => Deser Err");
                        }
                    }
                }
            }
            Ok(_) => break,
            Err(_shh) => {
                break;
            }
        }
    }

    let x = client.process(&None)?;

    if let Some(bmsg) = x.broker_response {
        let ser = to_stdvec_cobs(&bmsg).map_err(drop)?;
        stream.write_all(&ser).map_err(drop)?;
    }

    if let Some(cmsg) = x.client_response {
        println!("'{:?}': '{:?}'", cmsg.path, cmsg.payload);
    }

    Ok(())
}

fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    stream.set_nonblocking(true).unwrap();

    let mut client = Client::new(
        "cool-board",
        Version { major: 0, minor: 4, trivial: 1, misc: 123 },
        123
    );

    let mut current = vec![];

    while !client.is_connected() {
        processor(&mut stream, &mut client, &mut current).unwrap();
    }

    println!("Connected.");

    let path = Path::borrow_from_str("foo/bar/baz");
    let payload = b"henlo, welt!";

    let outgoing = client.subscribe(PubSubPath::Long(path.clone())).unwrap();
    let ser = to_stdvec_cobs(&outgoing).map_err(drop).unwrap();
    stream.write_all(&ser).map_err(drop).unwrap();

    while !client.is_subscribe_pending() {
        processor(&mut stream, &mut client, &mut current).unwrap();
    }

    println!("Subscribed.");

    let mut ctr = 0;
    let mut last_tx = Instant::now();

    while ctr < 10 {
        if last_tx.elapsed() >= Duration::from_secs(2) {
            last_tx = Instant::now();
            ctr += 1;

            let msg = client.publish(PubSubPath::Long(path.clone()), payload).unwrap();

            println!("Sending...");

            let ser = to_stdvec_cobs(&msg).map_err(drop).unwrap();
            stream.write_all(&ser).map_err(drop).unwrap();
        }

        processor(&mut stream, &mut client, &mut current).unwrap();
    }

    // stream.write(&connect).unwrap();

    // let path = "foo/bar/baz";
    // let payload = b"henlo, welt!";

    // let subscribe = to_stdvec_cobs(&Component::PubSub(PubSub {
    //     path: PubSubPath::Long(path),
    //     ty: PubSubType::Sub,
    // }))
    // .unwrap();
    // stream.write(&subscribe).unwrap();

    // for i in 30..40 {
    //     let publish = to_stdvec_cobs(&Component::PubSub(PubSub {
    //         path: PubSubPath::Long(path),
    //         ty: PubSubType::Pub { payload },
    //     }))
    //     .unwrap();
    //     stream.write(&publish).unwrap();
    //     let now = Instant::now();
    //     let mut scratch = [0u8; 1024];
    //     let mut current = vec![];
    //     while now.elapsed() < Duration::from_secs(2) {
    //         match stream.read(&mut scratch) {
    //             Ok(n) if n > 0 => {
    //                 current.extend_from_slice(&scratch[..n]);
    //                 while let Some(p) = current.iter().position(|c| *c == 0x00) {
    //                     let mut remainder = current.split_off(p + 1);
    //                     core::mem::swap(&mut remainder, &mut current);
    //                     let mut payload = remainder;
    //                     match from_bytes_cobs::<Arbitrator>(payload.as_mut_slice()) {
    //                         Ok(msg) => {
    //                             println!("  => Got {:?}", msg);
    //                         }
    //                         _ => {
    //                             println!("  => Deser Err");
    //                         }
    //                     }
    //                 }
    //             }
    //             _ => sleep(Duration::from_millis(50)),
    //         }
    //     }
    // }
}
