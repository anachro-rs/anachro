//! # The Anachro Protocol Client Library
//!
//! This crate is used by devices acting as a Client of the Anachro Protocol

#![no_std]

pub use {
    crate::{
        client::{Client, PUBLISH_SHORTCODE_OFFSET},
        client_io::{ClientIo, ClientIoError},
        table::{Table, TableError},
    },
    anachro_icd::{self, arbitrator::SubMsg, ManagedString, Path, PubSubPath, Version},
    postcard::{from_bytes, from_bytes_cobs, to_slice, to_slice_cobs},
    defmt::Format,
};

mod client;
mod client_io;
mod table;

/// The main Client error type
#[derive(Debug, PartialEq, Eq, Format)]
pub enum Error {
    NotActive,
    Busy,
    UnexpectedMessage,
    ClientIoError(ClientIoError),
}

impl From<ClientIoError> for Error {
    fn from(other: ClientIoError) -> Self {
        Error::ClientIoError(other)
    }
}

/// A message that has been received FROM the Broker, TO the Client
#[derive(Debug)]
pub struct RecvMsg<T: Table> {
    pub path: Path<'static>,
    pub payload: T,
}

/// A message to be sent TO the Broker, FROM the Client
#[derive(Debug)]
pub struct SendMsg<'a> {
    pub buf: &'a [u8],
    pub path: &'static str,
}
