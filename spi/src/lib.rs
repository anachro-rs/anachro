#[derive(Debug)]
pub enum Error {
    TxQueueFull,
    ToDo, // REMOVEME
    IncompleteTransaction(usize),
    ArbitratorHungUp,
}

pub type Result<T> = core::result::Result<T, Error>;

pub trait EncLogicLLComponent {
    /// Set the READY line low (active)
    fn notify_ready(&mut self) -> Result<()>;

    /// Set the READY line high (inactive)
    fn clear_ready(&mut self) -> Result<()>;

    /// Query whether the GO line is low (active)
    // TODO: just &self?
    fn is_go_active(&mut self) -> Result<bool>;

    /// Prepare data to be exchanged. The data MUST not be referenced
    /// until `complete_exchange` or `abort_exchange` has been called.
    ///
    /// NOTE: Data will not be sent until `trigger_exchange` has been
    /// called. This will automatically set the READY line if it is
    /// not already active.
    ///
    /// An error will be returned if an exchange is already in progress
    // TODO: `embedded-dma`?
    fn prepare_exchange(
        &mut self,
        data_out: *const u8,
        data_out_len: usize,
        data_in: *mut u8,
        data_in_max: usize,
    ) -> Result<()>;

    /// Actually begin exchanging data
    ///
    /// Will return an error if READY and GO are not active
    fn trigger_exchange(&mut self) -> Result<()>;

    /// Is a `exchange` action still in progress?
    fn is_exchange_active(&self) -> Result<bool>;

    /// Attempt to complete a `exchange` action.
    ///
    /// Returns `Ok(())` if the `exchange` completed successfully.
    ///
    /// If the exchange is successful and `clear_ready` is `true`,
    /// then the READY line will be cleared.
    ///
    /// Will return an error if the exchange is still in progress.
    /// If the exchange is still in progress, `clear_ready` is ignored.
    ///
    /// Use `abort_exchange` to force the exchange to completion even
    /// if it is still in progress.
    fn complete_exchange(&mut self, clear_ready: bool) -> Result<usize>;

    /// Stop the `exchange` action immediately
    ///
    /// Returns `Ok(())` if the exchange had already been completed.
    ///
    /// In all cases, the READY line will be cleared.
    ///
    /// If the exchange had not yet completed, an Error containing the
    /// number of successfully sent bytes will be returned.
    fn abort_exchange(&mut self) -> Result<usize>;
}

pub trait EncLogicLLArbitrator {
    /// Is the Component requesting a transaction?
    fn is_ready_active(&mut self) -> Result<bool>;

    /// Set the GO line active (low)
    fn notify_go(&mut self) -> Result<()>;

    /// Set the GO line inactive(high)
    fn clear_go(&mut self) -> Result<()>;

    /// Prepare data to be exchanged. The data MUST not be referenced
    /// until `complete_exchange` or `abort_exchange` has been called.
    ///
    /// This will automatically set the GO line if it is
    /// not already active.
    ///
    /// Will return an error if READY is not active
    ///
    /// This will begin sending once the Component begins clocking data
    ///
    /// An error will be returned if an exchange is already in progress
    // TODO: `embedded-dma`?
    fn prepare_exchange(
        &mut self,
        data_out: *const u8,
        data_out_len: usize,
        data_in: *mut u8,
        data_in_max: usize,
    ) -> Result<()>;

    /// Is a `exchange` action still in progress?
    fn is_exchange_active(&self) -> Result<bool>;

    /// Attempt to complete a `exchange` action.
    ///
    /// Returns `Ok(())` if the `exchange` completed successfully.
    ///
    /// If the exchange is successful and `clear_go` is `true`,
    /// then the GO line will be cleared.
    ///
    /// Will return an error if the exchange is still in progress.
    /// If the exchange is still in progress, `clear_go` is ignored.
    ///
    /// Use `abort_exchange` to force the exchange to completion even
    /// if it is still in progress.
    fn complete_exchange(&mut self, clear_go: bool) -> Result<usize>;

    /// Stop the `exchange` action immediately
    ///
    /// Returns `Ok(usize)` if the exchange had already been completed.
    ///
    /// In all cases, the GO line will be cleared.
    ///
    /// If the exchange had not yet completed, an Error containing the
    /// number of successfully sent bytes will be returned.
    fn abort_exchange(&mut self) -> Result<usize>;
}

pub trait EncLogicHLArbitrator {
    /// Place a message to be sent over SPI
    fn enqueue(&mut self, msg: &[u8]) -> Result<()>;

    /// Attempt to receive a message over SPI
    fn dequeue<'a>(&mut self, msg_out: &'a mut [u8]) -> Result<Option<&'a [u8]>>;

    /// Periodic poll. Should be called regularly (or on interrupts?)
    fn poll(&mut self) -> Result<()>;

    fn get_ll<LL: EncLogicLLArbitrator>(&mut self) -> &mut LL;
}

pub trait EncLogicHLComponent {
    /// Place a message to be sent over SPI
    fn enqueue(&mut self, msg: &[u8]) -> Result<()>;

    /// Attempt to receive a message over SPI
    fn dequeue<'a>(&mut self, msg_out: &'a mut [u8]) -> Result<Option<&'a [u8]>>;

    /// Periodic poll. Should be called regularly (or on interrupts?)
    fn poll(&mut self) -> Result<()>;

    fn get_ll<LL: EncLogicLLComponent>(&mut self) -> &mut LL;
}
