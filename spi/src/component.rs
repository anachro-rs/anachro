use crate::{Result, BBFullDuplex};

use bbqueue::{
    framed::{FrameGrantR, FrameGrantW},
    ArrayLength, BBBuffer,
};

use anachro_client::{
    ClientIo,
    ClientIoError,
    anachro_icd::{
        arbitrator::Arbitrator,
        component::Component,
    },
    from_bytes_cobs,
    to_slice_cobs,
};

pub trait EncLogicLLComponent {
    /// Process low level messages
    fn process(&mut self) -> Result<()>;

    /// Is the Component requesting a transaction?
    fn is_ready_active(&mut self) -> Result<bool>;

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


pub struct EncLogicHLComponent<LL, CT>
where
    CT: ArrayLength<u8>,
    LL: EncLogicLLComponent,
{
    ll: LL,
    outgoing_msgs: BBFullDuplex<CT>,
    incoming_msgs: BBFullDuplex<CT>,
    out_grant: Option<FrameGrantR<'static, CT>>,
    in_grant: Option<FrameGrantW<'static, CT>>,
    out_buf: [u8; 4],
    sent_hdr: bool,
    triggered: bool,

    // NOTE: This is the grant from the incoming queue, used to return
    // messages up the protocol stack. By holding the grant HERE, we tie
    // the zero-copy message to the borrow of self, which means that
    // the higher levels of the stack MUST release the incoming message
    // before they do anything else with Self. We then just drop the grant
    // the next time the user asks us to receive a message, which works,
    // because they've already let go of the reference to the data contained
    // by the grant (by releasing the borrow of Self).
    //
    // I *think* this is the best we can do without radically re-architecting
    // how the entire stack works, switching to something like heapless::Pool,
    // or totally reconsidering zero-copy entirely.
    current_grant: Option<FrameGrantR<'static, CT>>,
}

impl<LL, CT> EncLogicHLComponent<LL, CT>
where
    CT: ArrayLength<u8>,
    LL: EncLogicLLComponent,
{
    pub fn new(
        ll: LL,
        outgoing: &'static BBBuffer<CT>,
        incoming: &'static BBBuffer<CT>
    ) -> Result<Self> {
        Ok(EncLogicHLComponent {
            ll,
            outgoing_msgs: BBFullDuplex::new(outgoing)?,
            incoming_msgs: BBFullDuplex::new(incoming)?,
            out_buf: [0u8; 4],
            out_grant: None,
            in_grant: None,
            sent_hdr: false,
            triggered: false,
            current_grant: None,
        })
    }

    pub fn dequeue(&mut self) -> Option<FrameGrantR<'static, CT>> {
        self.incoming_msgs.cons.read()
    }

    pub fn enqueue(&mut self, msg: &[u8]) -> Result<()> {
        let len = msg.len();
        let mut wgr = self.outgoing_msgs.prod.grant(len)?;
        wgr.copy_from_slice(msg);
        wgr.commit(len);
        Ok(())
    }

    pub fn poll(&mut self) -> Result<()> {
        self.ll.process()?;

        if !self.ll.is_exchange_active().unwrap() {
            // TODO: We probably also should occasionally just
            // poll for incoming messages, even when we don't
            // have any outgoing messages to process
            if let Some(msg) = self.outgoing_msgs.cons.read() {
                if !self.sent_hdr {
                    let igr_ptr;
                    match self.incoming_msgs.prod.grant(4) {
                        Ok(mut igr) => {
                            igr_ptr = igr.as_mut_ptr();
                            assert!(self.in_grant.is_none(), "Why do we already have an in grant?");
                            self.in_grant = Some(igr);
                        }
                        Err(_) => {
                            todo!("Handle insufficient size available for incoming");
                        }
                    }
                    // TODO: Do I want to save the grant here? I just need to "peek" to
                    // get the header values

                    // println!("Starting exchange, header!");
                    self.out_buf = (msg.len() as u32).to_le_bytes();

                    self.ll
                        .prepare_exchange(self.out_buf.as_ptr(), 4, igr_ptr, 4)
                        .unwrap();
                } else {
                    let out_ptr = msg.as_ptr();
                    let out_len = msg.len();
                    self.out_grant = Some(msg);

                    let amt = match self.in_grant.take() {
                        Some(igr) => {
                            // Note: Drop IGR without commit by taking
                            assert_eq!(igr.len(), 4, "wrong header igr?");
                            let mut buf = [0u8; 4];
                            buf.copy_from_slice(&igr);
                            u32::from_le_bytes(buf) as usize
                        }
                        None => {
                            panic!("Why don't we have a header igr?");
                        }
                    };

                    let in_ptr;
                    self.in_grant = match self.incoming_msgs.prod.grant(amt) {
                        Ok(mut igr) => {
                            in_ptr = igr.as_mut_ptr();
                            Some(igr)
                        }
                        Err(_) => {
                            todo!("Handle insufficient size of igr")
                        }
                    };

                    // println!("Starting exchange, data!");
                    self.ll
                        .prepare_exchange(out_ptr, out_len, in_ptr, amt)
                        .unwrap();
                }
            }
        } else {
            if !self.triggered {
                if self.ll.is_go_active().unwrap() {
                    // println!("triggering!");
                    self.ll.trigger_exchange().unwrap();
                    self.triggered = true;
                }
            } else {
                if let Ok(false) = self.ll.is_go_active() {
                    // println!("aborting!");
                    self.ll.abort_exchange().ok();
                }
                match self.ll.complete_exchange(self.sent_hdr) {
                    Err(_) => {}
                    Ok(amt) => {
                        // println!("got {:?}!", in_buf);
                        if self.sent_hdr {
                            // Note: relax this for "empty" requests to the arbitrator
                            assert!(self.in_grant.is_some(), "Why no in grant on completion?");

                            if let Some(igr) = self.in_grant.take() {
                                if amt != 0 {
                                    igr.commit(amt);
                                }
                            }

                            if let Some(ogr) = self.out_grant.take() {
                                ogr.release();
                            }
                        }
                        self.triggered = false;
                        self.sent_hdr = !self.sent_hdr;
                    }
                }
            }
        }

        Ok(())
    }
}

impl<LL, CT> ClientIo for EncLogicHLComponent<LL, CT>
where
    CT: ArrayLength<u8>,
    LL: EncLogicLLComponent,
{
    /// Attempt to receive one message FROM the Arbitrator/Broker, TO the Client
    fn recv(&mut self) -> core::result::Result<Option<Arbitrator>, ClientIoError> {
        // Note: This is fine because we've made the grant (if any) auto-release,
        // so the data will be released on drop.
        //
        // Todo: In the future I wonder if I can make this drop happen sooner,
        // e.g. when the borrow of &self is released, to free up space faster
        self.current_grant = None;
        match self.dequeue() {
            Some(mut msg) => {
                // Set message to automatically release on drop
                msg.auto_release(true);
                self.current_grant = Some(msg);
                let sbr = self.current_grant.as_mut().unwrap();

                // TODO: Cobs encoding at this level is probably not super necessary,
                // because for now we only handle one message exchange at a time. In the
                // future, it might be possible to pack multiple datagrams together into
                // a single frame. But for now, we only handle one.
                match from_bytes_cobs(sbr) {
                    Ok(deser) => {
                        Ok(deser)
                    }
                    Err(_) => {
                        Err(ClientIoError::ParsingError)
                    }
                }
            }
            None => return Ok(None),
        }
    }

    /// Attempt to send one message TO the Arbitrator/Broker, FROM the Client
    fn send(&mut self, msg: &Component) -> core::result::Result<(), ClientIoError> {
        // HACK: Actual sizing. /4 is based on nothing actually
        match self.outgoing_msgs.prod.grant(CT::to_usize() / 4) {
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
