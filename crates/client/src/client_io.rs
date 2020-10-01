//! The Client Io Interface
//!
//! This module defines the `ClientIo` trait, which is used for describing
//! how to send and receive messages "over the wire". The Anachro Protocol
//! is transport-agnostic, which means you could send messages:
//!
//! * Over a packet radio, using the framing provided by the radio
//! * Over a serial port, using COBS for message framing
//! * Over a TCP port, using COBS for message framing
//! * Over a UDP port, using UDP frames
//! * Over a shared memory interface, using some other framing mechanism
//! * Literally any other way you can think of to shuffle bytes around
//!
//! Implementors of this trait are responsible for serializing the data
//! appropriately, and potentially buffering multiple packets to be sent
//! when necessary. Implementors may choose to immediately send and receive,
//! Or to enqueue/dequeue messages upon request.

use anachro_icd::{arbitrator::Arbitrator, component::Component};
use defmt::Format;

/// The Error type of the ClientIo interface
#[derive(Debug, PartialEq, Eq, Format)]
pub enum ClientIoError {
    /// The ClientIo implementor failed to deserialize an incoming message
    ParsingError,

    /// The ClientIo implementor does not have any data to give at the moment
    ///
    /// TODO: This should probably be removed and we should just return `Ok(None)`
    NoData,

    /// The ClientIo is unable to send a packet, as the interface is full/busy
    OutputFull,
}

/// A trait for defining the IO layer for a given client
pub trait ClientIo {
    /// Attempt to receive one message FROM the Arbitrator/Broker, TO the Client
    fn recv(&mut self) -> Result<Option<Arbitrator>, ClientIoError>;

    /// Attempt to send one message TO the Arbitrator/Broker, FROM the Client
    fn send(&mut self, msg: &Component) -> Result<(), ClientIoError>;
}
