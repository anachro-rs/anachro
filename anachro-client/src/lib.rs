#![no_std]

pub use {
    crate::{
        client::{Client, ClientState, PUBLISH_SHORTCODE_OFFSET},
        client_io::{ClientIo, ClientIoError},
        table::{Table, TableError},
    },
    anachro_icd::{self, arbitrator::SubMsg, ManagedString, Path, PubSubPath, Version},
    postcard::{from_bytes, to_slice},
};

mod client;
mod client_io;
mod table;

#[derive(Debug, PartialEq)]
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

impl ClientState {
    pub(crate) fn as_active(&self) -> Result<(), Error> {
        match self {
            ClientState::Active => Ok(()),
            _ => Err(Error::NotActive),
        }
    }
}

#[derive(Debug)]
pub struct RecvMsg<T: Table> {
    pub path: Path<'static>,
    pub payload: T,
}

#[derive(Debug)]
pub struct SendMsg<'a> {
    pub buf: &'a [u8],
    pub path: &'static str,
}
