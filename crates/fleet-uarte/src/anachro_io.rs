use crate::app::UarteApp;
use crate::cobs_buf::{Buffer, SimpleResult};
use bbqueue::ArrayLength;

use anachro_client::{to_slice_cobs, ClientIo, ClientIoError};
use anachro_icd::{arbitrator::Arbitrator, component::Component, Uuid};
use anachro_server::{from_bytes_cobs, Request, ServerIoError, ServerIoIn};

pub struct AnachroUarte<OutgoingLen, IncomingLen, BufferLen>
where
    OutgoingLen: ArrayLength<u8>,
    IncomingLen: ArrayLength<u8>,
    BufferLen: ArrayLength<u8>,
{
    app: UarteApp<OutgoingLen, IncomingLen>,
    buf: Buffer<BufferLen>,
    uuid: Uuid,
}

impl<OutgoingLen, IncomingLen, BufferLen> AnachroUarte<OutgoingLen, IncomingLen, BufferLen>
where
    OutgoingLen: ArrayLength<u8>,
    IncomingLen: ArrayLength<u8>,
    BufferLen: ArrayLength<u8>,
{
    pub fn new(
        app: UarteApp<OutgoingLen, IncomingLen>,
        buf: Buffer<BufferLen>,
        uuid: Uuid,
    ) -> Self {
        Self { app, buf, uuid }
    }

    pub fn enqueue(&mut self, out: &[u8]) -> Result<(), ()> {
        let mut grant = self.app.write_grant(out.len()).map_err(drop)?;
        grant.copy_from_slice(out);
        grant.commit(out.len());
        Ok(())
    }

    pub fn dequeue<'a>(&'a mut self) -> Result<Option<&'a mut [u8]>, ()> {
        loop {
            if let Ok(rgr) = self.app.read() {
                let len = rgr.len();
                match self.buf.feed_simple(&rgr) {
                    SimpleResult::Consumed => {
                        rgr.release(len);
                    }
                    SimpleResult::OverFull(remaining) => {
                        let len_rem = remaining.len();
                        rgr.release(len - len_rem);
                    }
                    SimpleResult::Success { data, .. } => {
                        let len = data.len();
                        rgr.release(len);

                        // TODO: We *SHOULD* be able to just return `data` here, but
                        // borrow checker is sad. We know that the buffer always matches
                        return Ok(Some(&mut self.buf.buf.as_mut_slice()[..len]));
                    }
                }
            } else {
                return Ok(None);
            }
        }
    }
}

impl<OutgoingLen, IncomingLen, BufferLen> ClientIo
    for AnachroUarte<OutgoingLen, IncomingLen, BufferLen>
where
    OutgoingLen: ArrayLength<u8>,
    IncomingLen: ArrayLength<u8>,
    BufferLen: ArrayLength<u8>,
{
    /// Attempt to receive one message FROM the Arbitrator/Broker, TO the Client
    fn recv(&mut self) -> Result<Option<Arbitrator>, ClientIoError> {
        match self.dequeue() {
            Ok(Some(payload)) => match postcard::from_bytes_cobs::<Arbitrator>(payload) {
                Ok(t) => Ok(Some(t)),
                Err(_) => Err(ClientIoError::ParsingError),
            },
            Ok(None) => Ok(None),
            Err(()) => {
                // TODO: ehh.
                Err(ClientIoError::ParsingError)
            }
        }
    }

    /// Attempt to send one message TO the Arbitrator/Broker, FROM the Client
    fn send(&mut self, msg: &Component) -> Result<(), ClientIoError> {
        // HACK: Actual sizing. /4 is based on nothing actually
        match self.app.write_grant(BufferLen::to_usize() / 4) {
            Ok(mut wgr) => {
                match to_slice_cobs(msg, &mut wgr) {
                    Ok(amt) => {
                        let len = amt.len();
                        wgr.commit(len);
                        Ok(())
                    }
                    Err(_e) => {
                        // TODO: See hack above, might not really be a parsing error
                        Err(ClientIoError::ParsingError)
                    }
                }
            }
            Err(_e) => Err(ClientIoError::OutputFull),
        }
    }
}

impl<OutgoingLen, IncomingLen, BufferLen> ServerIoIn
    for AnachroUarte<OutgoingLen, IncomingLen, BufferLen>
where
    OutgoingLen: ArrayLength<u8>,
    IncomingLen: ArrayLength<u8>,
    BufferLen: ArrayLength<u8>,
{
    fn recv<'a, 'b: 'a>(&'b mut self) -> Result<Option<Request<'b>>, ServerIoError> {
        let uuid = self.uuid.clone();
        match self.dequeue() {
            Ok(Some(payload)) => match from_bytes_cobs::<Component>(payload) {
                Ok(t) => Ok(Some(Request {
                    source: uuid,
                    msg: t,
                })),
                Err(_) => Err(ServerIoError::DeserializeFailure),
            },
            Ok(None) => Ok(None),
            Err(()) => {
                // TODO: ehh.
                Err(ServerIoError::DeserializeFailure)
            }
        }
    }
}

// pub struct UarteApp<OutgoingLen, IncomingLen>
// where
//     OutgoingLen: ArrayLength<u8>,
//     IncomingLen: ArrayLength<u8>,
// {

// #[derive(Default)]
// pub struct Buffer<N: ArrayLength<u8>> {
//     buf: GenericArray<u8, N>,
//     idx: usize,
// }
