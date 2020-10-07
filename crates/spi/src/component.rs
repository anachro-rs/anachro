use crate::{BBFullDuplex, Error, Result};

use bbqueue::{
    framed::{FrameGrantR, FrameGrantW},
    ArrayLength, BBBuffer,
};

use anachro_client::{
    anachro_icd::{arbitrator::Arbitrator, component::Component},
    from_bytes_cobs, to_slice_cobs, ClientIo, ClientIoError,
};

use groundhog::RollingTimer;

const T_MIN_US: u32 = 1000;

pub trait EncLogicLLComponent {
    /// Process low level messages
    fn process(&mut self) -> Result<()>;

    /// Set the CSn line low (active)
    fn notify_csn(&mut self) -> Result<()>;

    /// Set the CSn line high (inactive)
    fn clear_csn(&mut self) -> Result<()>;

    /// Query whether the GO line is low (active)
    // TODO: just &self?
    fn is_go_active(&mut self) -> Result<bool>;

    /// Prepare data to be exchanged. The data MUST not be referenced
    /// until `complete_exchange` or `abort_exchange` has been called.
    ///
    /// An error will be returned if an exchange is already in progress
    // TODO: `embedded-dma`?
    fn begin_exchange(
        &mut self,
        data_out: *const u8,
        data_out_len: usize,
        data_in: *mut u8,
        data_in_max: usize,
    ) -> Result<()>;

    /// Is a `exchange` action still in progress?
    fn is_exchange_active(&self) -> Result<bool>;

    /// Attempt to complete an `exchange` action.
    ///
    /// Returns `Ok(())` if the `exchange` completed successfully.
    ///
    /// Will return an error if the exchange is still in progress.
    ///
    /// Use `abort_exchange` to force the exchange to completion even
    /// if it is still in progress.
    fn complete_exchange(&mut self) -> Result<usize>;

    /// Stop the `exchange` action immediately
    ///
    /// Returns `Ok(())` if the exchange had already been completed.
    ///
    /// If the exchange had not yet completed, an Error containing the
    /// number of successfully sent bytes will be returned.
    fn abort_exchange(&mut self) -> Result<usize>;
}

#[derive(Debug)]
enum SendingState<CT, RT>
where
    CT: ArrayLength<u8>,
    RT: RollingTimer,
{
    Idle,
    HeaderStart(RT::Tick),
    HeaderXfer,
    HeaderComplete(RT::Tick),
    BodyStart(RT::Tick),
    BodyXfer(
        Option<FrameGrantR<'static, CT>>,
        Option<FrameGrantW<'static, CT>>,
    ),
    BodyComplete(RT::Tick),
}

pub struct EncLogicHLComponent<LL, CT, RT>
where
    CT: ArrayLength<u8>,
    LL: EncLogicLLComponent,
    RT: RollingTimer<Tick = u32>,
{
    ll: LL,
    outgoing_msgs: BBFullDuplex<CT>,
    incoming_msgs: BBFullDuplex<CT>,
    smol_buf_in: [u8; 4],
    smol_buf_out: [u8; 4],
    send_state: SendingState<CT, RT>,
    timer: RT,

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

impl<LL, CT, RT> EncLogicHLComponent<LL, CT, RT>
where
    CT: ArrayLength<u8>,
    LL: EncLogicLLComponent,
    RT: RollingTimer<Tick = u32>,
{
    pub fn new(
        ll: LL,
        timer: RT,
        outgoing: &'static BBBuffer<CT>,
        incoming: &'static BBBuffer<CT>,
    ) -> Result<Self> {
        Ok(EncLogicHLComponent {
            ll,
            outgoing_msgs: BBFullDuplex::new(outgoing)?,
            incoming_msgs: BBFullDuplex::new(incoming)?,
            smol_buf_in: [0u8; 4],
            smol_buf_out: [0u8; 4],
            send_state: SendingState::Idle,
            timer,
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
        // First things first, set the current state to idle. If we bail out
        // at any point after this, we'll just be sitting back in the idle state
        let old_state = core::mem::replace(&mut self.send_state, SendingState::Idle);

        // Let the ll driver chew on packets
        self.ll.process()?;

        // Is the go line (still) active?
        let go_active = match self.ll.is_go_active() {
            Ok(go) => go,
            Err(_e) => {
                // TODO: error handling
                self.ll.clear_csn()?;
                return Ok(());
            }
        };

        // Is there an exchange still in progress?
        let exchange_active = match self.ll.is_exchange_active() {
            Ok(act) => act,
            Err(_e) => {
                // TODO: error handling
                self.ll.clear_csn()?;
                return Ok(());
            }
        };

        // Someone hung up, or we aren't active
        if !go_active {
            if exchange_active {
                defmt::warn!("Aborting active exchange!");
                self.ll.abort_exchange().ok();
            }
            self.ll.clear_csn()?;
            return Ok(());
        }

        let completed_exchange = if !exchange_active {
            None
        } else {
            match self.ll.complete_exchange() {
                Ok(amt) => {
                    defmt::info!("Exchange completed: {:?} bytes", amt);
                    Some(amt)
                },
                Err(Error::TransactionBusy) => None,
                Err(_e) => {
                    defmt::error!("Exchange error! Aborting exchange");
                    self.ll.abort_exchange().ok();
                    self.ll.clear_csn()?;
                    return Ok(());
                }
            }
        };

        // TODO: Actually complete exchange, currently using exchange_active wrong

        self.send_state = match old_state {
            SendingState::Idle => {
                debug_assert!(!exchange_active);

                defmt::info!("Component: Idle -> HeaderStart");

                self.ll.notify_csn()?;
                SendingState::HeaderStart(self.timer.get_ticks())
            }
            SendingState::HeaderStart(t_start) => {
                debug_assert!(!exchange_active);

                if self.timer.micros_since(t_start) > T_MIN_US {
                    let next_amt = self
                        .outgoing_msgs
                        .cons
                        .read()
                        .map(|gr| gr.len())
                        .unwrap_or(0);

                    self.smol_buf_in = (0u32).to_le_bytes();
                    self.smol_buf_out = (next_amt as u32).to_le_bytes();

                    self.ll.begin_exchange(
                        self.smol_buf_out.as_ptr(),
                        4,
                        self.smol_buf_in.as_mut_ptr(),
                        4,
                    )?;

                    defmt::info!("Component: HeaderStart -> HeaderXfer");

                    SendingState::HeaderXfer
                } else {
                    SendingState::HeaderStart(t_start)
                }
            }
            SendingState::HeaderXfer => {
                if let Some(amt) = completed_exchange {
                    self.ll.clear_csn()?;

                    if amt != 4 {
                        defmt::error!("Header size mismatch?");
                        return Ok(());
                    }

                    let amt_in = u32::from_le_bytes(self.smol_buf_in);
                    let amt_out = u32::from_le_bytes(self.smol_buf_out);

                    if (amt_in == 0) && (amt_out == 0) {
                        defmt::info!("Component: HeaderXfer -> Idle");
                        // Nothing to do here!
                        return Ok(());
                    }

                    defmt::info!("Component: HeaderXfer -> HeaderComplete");

                    SendingState::HeaderComplete(self.timer.get_ticks())
                } else {
                    SendingState::HeaderXfer
                }
            }
            SendingState::HeaderComplete(t_start) => {
                debug_assert!(!exchange_active);

                if self.timer.micros_since(t_start) > T_MIN_US {
                    self.ll.notify_csn()?;

                    defmt::info!("Component: HeaderComplete -> BodyStart");

                    SendingState::BodyStart(self.timer.get_ticks())
                } else {
                    SendingState::HeaderComplete(t_start)
                }
            }
            SendingState::BodyStart(t_start) => {
                debug_assert!(!exchange_active);

                if self.timer.micros_since(t_start) > T_MIN_US {
                    let out_ptr;
                    let out_len;
                    let in_ptr;
                    let in_len;

                    let amt_in = u32::from_le_bytes(self.smol_buf_in);
                    let amt_out = u32::from_le_bytes(self.smol_buf_out);

                    defmt::error!("Header in: {:?}, header out: {:?}", amt_in, amt_out);

                    if amt_in > 4096 {
                        defmt::error!("Illogical size!");
                        self.ll.clear_csn()?;
                        return Ok(());
                    }

                    let wgr = if amt_in == 0 {
                        in_ptr = self.smol_buf_in.as_mut_ptr();
                        in_len = 0;
                        None
                    } else {
                        defmt::error!("How about {:?}", amt_in);
                        let aiau = amt_in as usize;
                        defmt::error!("Reqesting {:?}", aiau);
                        let mut wgr = self.incoming_msgs.prod.grant(aiau).unwrap(); //?;
                        in_len = amt_in as usize;
                        in_ptr = wgr.as_mut_ptr();
                        Some(wgr)
                    };

                    let rgr = match self.outgoing_msgs.cons.read() {
                        Some(rgr) => {
                            debug_assert!(
                                rgr.len() == u32::from_le_bytes(self.smol_buf_out) as usize
                            );

                            defmt::error!("Sending: {:?}", &rgr[..]);

                            out_len = rgr.len();
                            out_ptr = rgr.as_ptr();

                            Some(rgr)
                        }
                        None => {
                            debug_assert!(0 == u32::from_le_bytes(self.smol_buf_out));

                            out_len = 0;
                            out_ptr = self.smol_buf_out.as_ptr();

                            None
                        }
                    };

                    defmt::info!("Starting Body transfer. Expecting rx: {:?} tx: {:?}", in_len, out_len);

                    self.ll.begin_exchange(out_ptr, out_len, in_ptr, in_len)?;

                    defmt::info!("Component: BodyStart -> BodyXfer");

                    SendingState::BodyXfer(rgr, wgr)
                } else {
                    SendingState::BodyStart(t_start)
                }
            }
            SendingState::BodyXfer(fgr, fgw) => {
                if let Some(amt) = completed_exchange {
                    // Complete body transfer?
                    // Go to Body Complete
                    self.ll.clear_csn()?;

                    if let Some(fgr) = fgr {
                        // This is the outgoing buffer
                        fgr.release();
                    }

                    if let Some(fgw) = fgw {
                        // This is the incoming buffer
                        defmt::error!("Got message: {:?}", &fgw[..amt]);
                        fgw.commit(amt);
                    }

                    defmt::info!("Component: BodyXfer -> BodyComplete");

                    SendingState::BodyComplete(self.timer.get_ticks())
                } else {
                    SendingState::BodyXfer(fgr, fgw)
                }
            }
            SendingState::BodyComplete(t_start) => {
                debug_assert!(!exchange_active);

                if self.timer.micros_since(t_start) > T_MIN_US {
                    self.ll.notify_csn()?;

                    defmt::info!("Component: BodyComplete -> HeaderStart");

                    SendingState::HeaderStart(self.timer.get_ticks())
                } else {
                    SendingState::BodyComplete(t_start)
                }
            }
        };

        Ok(())
    }
}

impl<LL, CT, RT> ClientIo for EncLogicHLComponent<LL, CT, RT>
where
    CT: ArrayLength<u8>,
    LL: EncLogicLLComponent,
    RT: RollingTimer<Tick = u32>,
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
