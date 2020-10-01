#![cfg_attr(not(feature = "tcp"), no_std)]

use bbqueue::{
    framed::{FrameConsumer, FrameProducer},
    ArrayLength, BBBuffer, Error as BBError,
};
use defmt::Format;

pub mod arbitrator;
pub mod component;

#[derive(Debug, Format)]
pub enum Error {
    TxQueueFull,
    IncompleteTransaction(usize),
    ArbitratorHungUp,
    InitializationFailed,

    // Consider swallowing the error to prevent defmt leakage
    BBQueueError(BBError),

    // E_WOULD_BLOCK
    TransactionBusy,

    TransactionAborted,

    ConnectionFailure,

    // We tried to prep a message, but the component had not
    // marked itself as ready
    ComponentNotReady,

    // We were in an unexpected state when an event happened.
    // This likely is indicative of an internal error.
    IncorrectState,

    // Something went wrong with my use of `Unstable` and buffer swaps
    UnstableFailure,

    // e-h gpio error
    GpioError,

    // e-h spi error
    SpiError,
}

impl From<BBError> for Error {
    fn from(b: BBError) -> Self {
        Error::BBQueueError(b)
    }
}

pub type Result<T> = core::result::Result<T, Error>;

pub(crate) struct BBFullDuplex<CT>
where
    CT: ArrayLength<u8>,
{
    pub(crate) prod: FrameProducer<'static, CT>,
    pub(crate) cons: FrameConsumer<'static, CT>,
}

impl<CT> BBFullDuplex<CT>
where
    CT: ArrayLength<u8>,
{
    pub(crate) fn new(a: &'static BBBuffer<CT>) -> Result<BBFullDuplex<CT>> {
        let (prod, cons) = a
            .try_split_framed()
            .map_err(|_| Error::InitializationFailed)?;

        Ok(BBFullDuplex { prod, cons })
    }
}
