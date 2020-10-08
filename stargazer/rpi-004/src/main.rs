mod comms;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;

use std::thread::{sleep, spawn};
use std::time::Duration;


fn main() -> Result<()> {
    let mut modem = comms::CommsCtx::new("/dev/ttyUSB0")?;

    loop {
        match modem.poll() {
            Ok(_) => {
                sleep(Duration::from_millis(50));
            }
            Err(e) => {
                println!("boop modem: {:?}", e);
                std::process::exit(1);
            }
        }
    }
}
