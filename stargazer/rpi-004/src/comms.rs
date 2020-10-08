use crate::Result;
use stargazer_icd::{DisplayTable, Keypress};
use serialport::prelude::*;
use std::{
    io::prelude::*,
    time::Duration,
};

pub struct CommsCtx {
    uart: UartAnachro,
    client: Client,
    ever_connected: bool,
}

use anachro_icd::{
    arbitrator::Arbitrator,
    component::Component,
    Version,
};
use anachro_client::{ClientIo, ClientIoError, Client, Error};
use postcard::{from_bytes_cobs, to_stdvec_cobs};

struct UartAnachro {
    port: Box<dyn SerialPort>,
    scratch: Vec<u8>,
    current: Option<Vec<u8>>
}


impl ClientIo for UartAnachro {
    fn recv(&mut self) -> core::result::Result<Option<Arbitrator>, ClientIoError> {
        let mut scratch = [0u8; 1024];

        loop {
            match self.port.read(&mut scratch) {
                Ok(n) if n > 0 => {
                    self.scratch.extend_from_slice(&scratch[..n]);

                    if let Some(p) = self.scratch.iter().position(|c| *c == 0x00) {
                        let mut remainder = self.scratch.split_off(p + 1);
                        core::mem::swap(&mut remainder, &mut self.scratch);
                        self.current = Some(remainder);

                        if let Some(ref mut payload) = self.current {
                            if let Ok(msg) = from_bytes_cobs::<Arbitrator>(payload.as_mut_slice()) {
                                // println!("GIVING: {:?}", msg);
                                return Ok(Some(msg));
                            }
                        }

                        return Err(ClientIoError::ParsingError);
                    }
                }
                Ok(_) => return Ok(None),
                Err(_) => return Ok(None),
            }
        }
    }
    fn send(&mut self, msg: &Component) -> core::result::Result<(), ClientIoError> {
        // println!("SENDING: {:?}", msg);
        let ser = to_stdvec_cobs(msg).map_err(|_| ClientIoError::ParsingError)?;
        self.port
            .write_all(&ser)
            .map_err(|_| ClientIoError::OutputFull)?;
        Ok(())
    }
}

impl CommsCtx {
    pub fn new(uart: &str) -> Result<Self> {
        let mut settings: SerialPortSettings = Default::default();
        // TODO: Should be configurable settings
        settings.timeout = Duration::from_millis(50);
        settings.baud_rate = 1_000_000;

        let port = match serialport::open_with_settings(uart, &settings) {
            Ok(port) => port,
            Err(e) => {
                eprintln!("Failed to open \"{}\". Error: {}", uart, e);
                ::std::process::exit(1);
            }
        };

        let client = Client::new(
            "rpi-004",
            Version {
                major: 0,
                minor: 4,
                trivial: 1,
                misc: 123,
            },
            987,
            DisplayTable::sub_paths(),
            DisplayTable::pub_paths(),
            Some(5),
        );

        Ok(Self {
            uart: UartAnachro {
                port,
                scratch: vec![],
                current: None,
            },
            client,
            ever_connected: false,
        })
    }

    pub fn poll(&mut self) -> Result<()> {

        loop {
            let msg = match self.client.process_one::<_, DisplayTable>(&mut self.uart) {
                Ok(Some(msg)) => {
                    match msg.payload {
                        DisplayTable::Key( Keypress { character }) => {
                            print!("{}", character);
                            std::io::stdout().flush().ok().expect("Could not flush stdout");
                        }
                        _ => {}
                    }
                },
                Ok(None) => {
                    break;
                }
                Err(Error::ClientIoError(ClientIoError::NoData)) => {
                    break;
                }
                Err(e) => {
                    println!("error: {:?}", e);
                    return Err("errr oohhh".into());
                },
            };

            // for route in self.routes.iter_mut() {
            //     for path in route.paths.iter() {
            //         if msg.path.as_str() == *path {
            //             route.comms.tx.send(msg.payload.clone()).ok();
            //         }
            //     }
            // }
        }

        if self.client.is_connected() {
            if !self.ever_connected {
                self.ever_connected = true;
                println!("Connected!\n\n");
            }

            // for route in self.routes.iter_mut() {
            //     while let Ok(msg) = route.comms.rx.try_recv() {
            //         let mut buf = [0u8; 1024];
            //         let pubby = msg.serialize(&mut buf).map_err(|_| "arg")?;
            //         self.client.publish(
            //             &mut self.uart,
            //             pubby.path,
            //             pubby.buf,
            //         ).map_err(|_| "blarg")?;
            //     }
            // }
        }


        Ok(())
    }
}
