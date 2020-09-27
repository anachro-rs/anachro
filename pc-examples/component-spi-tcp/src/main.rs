use anachro_spi::component::EncLogicHLComponent;
use anachro_spi_tcp::TcpSpiComLL;
use std::net::TcpStream;

use std::time::{Duration, Instant};

use bbqueue::{
    consts::*,
    BBBuffer, ConstBBBuffer,
};

static BB_OUT: BBBuffer<U2048> = BBBuffer( ConstBBBuffer::new() );
static BB_INP: BBBuffer<U2048> = BBBuffer( ConstBBBuffer::new() );

fn main() {
    let stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    stream.set_nonblocking(true).unwrap();

    println!("Component connected!");
    let mut com = EncLogicHLComponent::new(
        TcpSpiComLL::new(stream),
        &BB_OUT,
        &BB_INP
    ).unwrap();

    let mut last_tx = Instant::now();

    while let Ok(_) = com.poll() {
        while let Some(msg) = com.dequeue() {
            println!("==> Got HL msg: {:?}", &msg[..]);
            msg.release();
        }

        if last_tx.elapsed() > Duration::from_secs(5) {
            println!("==> Enqueuing!");
            com.enqueue(&[0x0F; 9]).unwrap();
            last_tx = Instant::now();
        }
    }
}
