use anachro_icd::{arbitrator::Arbitrator, component::Component};

#[derive(Debug, PartialEq)]
pub enum ClientIoError {
    ParsingError,
    NoData,
    OutputFull,
}

pub trait ClientIo {
    fn recv(&mut self) -> Result<Option<Arbitrator>, ClientIoError>;
    fn send(&mut self, msg: &Component) -> Result<(), ClientIoError>;
}
