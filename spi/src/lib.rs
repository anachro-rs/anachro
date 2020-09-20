use bbqueue::{
    framed::{FrameConsumer, FrameProducer},
    ArrayLength, BBBuffer, Error as BBError,
};

pub mod component;
pub mod arbitrator;

#[derive(Debug)]
pub enum Error {
    TxQueueFull,
    ToDo, // REMOVEME
    IncompleteTransaction(usize),
    ArbitratorHungUp,
    InitializationFailed,
    BBQueueError(BBError),
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
    pub(crate) fn new(
        a: &'static BBBuffer<CT>,
    ) -> Result<BBFullDuplex<CT>> {
        let (prod, cons) = a.try_split_framed().map_err(|_| Error::InitializationFailed)?;

        Ok(BBFullDuplex {
            prod,
            cons,
        })
    }
}
