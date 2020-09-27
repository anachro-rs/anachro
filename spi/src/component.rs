use crate::{Result, BBFullDuplex, Error};

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

#[derive(Debug)]
enum SendingState<CT>
where
    CT: ArrayLength<u8>
{
    Idle,
    DataHeader(FrameGrantR<'static, CT>),
    DataBody,
    EmptyHeader,
    EmptyBody,
}

impl<CT> PartialEq for SendingState<CT>
where
    CT: ArrayLength<u8>
{
    fn eq(&self, other: &SendingState<CT>) -> bool {
        match (self, other) {
            (SendingState::Idle, SendingState::Idle) => true,
            (SendingState::DataHeader(_), SendingState::DataHeader(_)) => true,
            (SendingState::DataBody, SendingState::DataBody) => true,
            (SendingState::EmptyHeader, SendingState::EmptyHeader) => true,
            (SendingState::EmptyBody, SendingState::EmptyBody) => true,
            _ => false,
        }
    }
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
    // sent_hdr: bool,
    triggered: bool,
    // empty_sending: bool,

    send_state: SendingState<CT>,

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
            // sent_hdr: false,
            triggered: false,
            current_grant: None,
            // empty_sending: false,
            send_state: SendingState::Idle,
        })
    }

    fn drop_grants(&mut self) {
        self.current_grant = None;
        self.out_grant = None;
        self.in_grant = None;
        self.triggered = false;
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

    fn setup_data(&mut self, msg: &FrameGrantR<'static, CT>) -> Result<()> {
        let igr_ptr;
        match self.incoming_msgs.prod.grant(4) {
            Ok(mut igr) => {
                igr_ptr = igr.as_mut_ptr();
                assert!(self.in_grant.is_none(), "Why do we already have an in grant?");
                self.in_grant = Some(igr);
            }
            Err(_) => {
                todo!("1 Handle insufficient size available for incoming");
            }
        }
        // TODO: Do I want to save the grant here? I just need to "peek" to
        // get the header values

        // println!("Starting exchange, header!");
        self.out_buf = (msg.len() as u32).to_le_bytes();

        self.ll
            .prepare_exchange(self.out_buf.as_ptr(), 4, igr_ptr, 4)
            .unwrap();

        Ok(())
    }

    fn setup_empty(&mut self) -> Result<()> {
        let igr_ptr;
        match self.incoming_msgs.prod.grant(4) {
            Ok(mut igr) => {
                igr_ptr = igr.as_mut_ptr();
                assert!(self.in_grant.is_none(), "Why do we already have an in grant?");
                self.in_grant = Some(igr);
            }
            Err(_) => {
                todo!("2 Handle insufficient size available for incoming");
            }
        }
        // TODO: Do I want to save the grant here? I just need to "peek" to
        // get the header values

        // println!("Starting exchange, header!");
        self.out_buf = (0u32).to_le_bytes();

        self.ll
            .prepare_exchange(self.out_buf.as_ptr(), 4, igr_ptr, 4)
            .unwrap();

        Ok(())
    }

    fn complete_data_header(&mut self, msg: FrameGrantR<'static, CT>) -> Result<SendingState<CT>> {
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

        Ok(SendingState::DataBody)
    }

    fn complete_empty_header(&mut self) -> Result<SendingState<CT>> {
        let out_ptr = self.out_buf.as_ptr();
        let out_len = 0;

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

        Ok(SendingState::EmptyBody)
    }

    pub fn poll(&mut self) -> Result<()> {
        // println!("Ding");
        self.ll.process()?;

        let exchange_active = match self.ll.is_exchange_active() {
            Ok(act) => act,
            Err(_e) => {
                // TODO: error handling
                return Ok(());
            }
        };

        let go_active = match self.ll.is_go_active() {
            Ok(go) => go,
            Err(_e) => {
                // TODO: error handling
                return Ok(());
            }
        };

        let old_state = core::mem::replace(&mut self.send_state, SendingState::Idle);

        self.send_state = match (exchange_active, go_active, self.triggered, old_state) {
            (false, false, false, SendingState::Idle) => {
                // println!("IDLE");
                if self.ll.is_go_active().is_err() {
                    // TODO: This is a hack
                    SendingState::Idle
                } else if let Some(msg) = self.outgoing_msgs.cons.read() {
                    self.setup_data(&msg)?;
                    SendingState::DataHeader(msg)
                } else {
                    self.setup_empty()?;
                    SendingState::EmptyHeader
                }
            }
            (_, _, _, SendingState::Idle) => {
                // println!("!IDLE");
                // This is an error.
                // Exch/Go/Trigger shouldn't be set while idle
                self.ll.abort_exchange().ok();
                self.drop_grants();
                SendingState::Idle // TODO: return Err?
            }
            (false, true, true, _state) => {
                // This is an error.
                self.ll.abort_exchange().ok();
                self.drop_grants();
                SendingState::Idle // TODO: return Err?
            }
            (false, _, _, _) => {
                // println!("ABORT1");
                // This is an error.
                // exchange should be active
                self.ll.abort_exchange().ok();
                self.drop_grants();
                SendingState::Idle // TODO: return Err?
            }
            (true, false, _, _) => {
                // println!("ABORT2");
                // This is an error. Go has fallen during exchange
                self.drop_grants();
                self.ll.abort_exchange().ok();
                SendingState::Idle // TODO: return Err?
            }
            (true, true, false, state) => {
                // println!("TRIGGER");
                // TRIGGER
                match state {
                    SendingState::Idle => {
                        return Err(Error::ToDo);
                    },
                    state => {
                        self.ll.trigger_exchange()?;
                        // println!("TRIGGERED");
                        self.triggered = true;
                        state
                    }
                }
            }
            (true, true, true, state) => {
                // Did we just finish sending a body?
                let body_done = match state {
                    SendingState::EmptyBody |  SendingState::DataBody => true,
                    _ => false,
                };

                match self.ll.complete_exchange(body_done) {
                    Err(_) => {
                        // Probably not an error, the other end just hung up
                        return Err(Error::ToDo);
                    }
                    Ok(amt) => {

                        if body_done {
                            assert!(self.in_grant.is_some(), "Why no in grant on completion?");

                            if let Some(igr) = self.in_grant.take() {
                                if amt != 0 {
                                    // println!("got {:?}!", &igr[..amt]);
                                    igr.commit(amt);
                                }
                            }

                            if let Some(ogr) = self.out_grant.take() {
                                ogr.release();
                            }
                        }
                        self.triggered = false;
                    }
                }


                match state {
                    SendingState::Idle => {
                        return Err(Error::ToDo);
                    },
                    SendingState::DataHeader(gr) => {
                        self.complete_data_header(gr)?
                    }
                    SendingState::DataBody => {
                        SendingState::Idle
                    }
                    SendingState::EmptyHeader => {
                        self.complete_empty_header()?
                    }
                    SendingState::EmptyBody => {
                        SendingState::Idle
                    }
                }
            },
        };

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
                // println!("recv: {:?}", &msg[..]);
                msg.auto_release(true);
                self.current_grant = Some(msg);
                let sbr = self.current_grant.as_mut().unwrap();

                // TODO: Cobs encoding at this level is probably not super necessary,
                // because for now we only handle one message exchange at a time. In the
                // future, it might be possible to pack multiple datagrams together into
                // a single frame. But for now, we only handle one.
                match from_bytes_cobs(sbr) {
                    Ok(deser) => {
                        // println!("yay! {:?}", deser);
                        Ok(Some(deser))
                    }
                    Err(_) => {
                        // println!("Parsing Error!");
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
