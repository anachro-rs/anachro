use crate::{BBFullDuplex, Error, Result};

use anachro_server::{
    anachro_icd::Uuid,
    // Response,
    from_bytes_cobs,
    Request,
    // ServerIoOut,
    ServerIoError,
    ServerIoIn,
};
use bbqueue::{
    framed::{FrameGrantR, FrameGrantW},
    ArrayLength, BBBuffer,
};

use groundhog::RollingTimer;

const T_WINDOW_US: u32 = 1_000_000;
const T_STEP_US: u32 = 500_000;

pub trait EncLogicLLArbitrator: Send {
    /// Process low level messages
    fn process(&mut self) -> Result<()>;

    // Is the GO line active?
    fn is_go_active(&mut self) -> Result<bool>;

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

    /// Has the exchange begun?
    ///
    /// Note: If your MCU doesn't give access to CSn state, then
    /// just return true
    fn has_exchange_begun(&self) -> Result<bool>;

    /// Is a `exchange` action still in progress?
    fn is_exchange_active(&self) -> Result<bool>;

    /// Attempt to complete a `exchange` action.
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
    /// Returns `Ok(usize)` if the exchange had already been completed.
    ///
    /// In all cases, the GO line will be cleared.
    ///
    /// If the exchange had not yet completed, an Error containing the
    /// number of successfully sent bytes will be returned.
    fn abort_exchange(&mut self) -> Result<usize>;
}

enum ArbState<RT, CT>
where
    RT: RollingTimer<Tick = u32>,
    CT: ArrayLength<u8>,
{
    Idle,
    HeaderStart {
        t_window: RT::Tick,
        t_step: RT::Tick,
    },
    HeaderPrepped {
        t_window: RT::Tick,
        t_step: RT::Tick,
    },
    HeaderXfer {
        t_window: RT::Tick,
        // TODO: max single transfer timer?
        // Based on 4 byte timing?
    },
    BodyPrepped {
        t_window: RT::Tick,
        t_step: RT::Tick,
        fgr: Option<FrameGrantR<'static, CT>>,
        fgw: Option<FrameGrantW<'static, CT>>,
    },
    BodyXfer {
        t_window: RT::Tick,
        fgr: Option<FrameGrantR<'static, CT>>,
        fgw: Option<FrameGrantW<'static, CT>>,
        // TODO: max single transfer timer?
        // Based on N byte timing?
    },
}

pub struct EncLogicHLArbitrator<LL, CT, RT>
where
    CT: ArrayLength<u8>,
    LL: EncLogicLLArbitrator,
    RT: RollingTimer<Tick = u32>,
{
    ll: LL,
    uuid: Uuid,
    outgoing_msgs: BBFullDuplex<CT>,
    incoming_msgs: BBFullDuplex<CT>,
    smol_buf_out: [u8; 4],
    smol_buf_in: [u8; 4],
    current_state: ArbState<RT, CT>,
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

impl<LL, CT, RT> EncLogicHLArbitrator<LL, CT, RT>
where
    CT: ArrayLength<u8>,
    LL: EncLogicLLArbitrator,
    RT: RollingTimer<Tick = u32>,
{
    pub fn new(
        uuid: Uuid,
        ll: LL,
        timer: RT,
        outgoing: &'static BBBuffer<CT>,
        incoming: &'static BBBuffer<CT>,
    ) -> Result<Self> {
        Ok(EncLogicHLArbitrator {
            ll,
            uuid,
            outgoing_msgs: BBFullDuplex::new(outgoing)?,
            incoming_msgs: BBFullDuplex::new(incoming)?,
            smol_buf_in: [0u8; 4],
            smol_buf_out: [0u8; 4],
            timer,
            current_state: ArbState::Idle,

            current_grant: None,
        })
    }

    pub fn dequeue(&mut self) -> Option<FrameGrantR<'static, CT>> {
        let ret = self.incoming_msgs.cons.read();
        if ret.is_some() {
            defmt::info!("Dequeuing a message");
        } else {
            defmt::trace!("No message to dequeue");
        }
        ret
    }

    // TODO: `enqueue_with` function or something for zero-copy grants
    pub fn enqueue(&mut self, msg: &[u8]) -> Result<()> {
        defmt::info!("enqueing message - {:?} bytes", msg.len());
        defmt::trace!("message: {:?}", msg);
        let len = msg.len();
        let mut wgr = self.outgoing_msgs.prod.grant(len)?;
        wgr.copy_from_slice(msg);
        wgr.commit(len);
        Ok(())
    }

    pub fn query_component(&mut self) -> Result<()> {
        if let ArbState::Idle = self.current_state {
            let now = self.timer.get_ticks();
            self.current_state = ArbState::HeaderStart {
                t_window: now,
                t_step: now,
            };
            defmt::info!("Arbitrator: Idle -> HeaderStart");
            Ok(())
        } else {
            Err(Error::IncorrectState)
        }
    }

    fn timeout_violated(&self, state: &ArbState<RT, CT>) -> bool {
        match state {
            ArbState::Idle => false,
            ArbState::HeaderStart { t_window, t_step }
            | ArbState::HeaderPrepped { t_window, t_step }
            | ArbState::BodyPrepped {
                t_window, t_step, ..
            } => {
                let window_bad = self.timer.micros_since(*t_window) > T_WINDOW_US;
                let step_bad = self.timer.micros_since(*t_step) > T_STEP_US;
                if window_bad {
                    defmt::warn!("Window timeout!");
                }
                if step_bad {
                    defmt::warn!("Step timeout!");
                }
                window_bad || step_bad
            }
            ArbState::HeaderXfer { t_window } | ArbState::BodyXfer { t_window, .. } => {
                let window_bad = self.timer.micros_since(*t_window) > T_WINDOW_US;
                if window_bad {
                    defmt::warn!("Window timeout!");
                }
                window_bad
            }
        }
    }

    pub fn poll(&mut self) -> Result<()> {
        defmt::trace!("Polling...");
        self.ll.process()?;

        let mut old_state = ArbState::Idle;
        core::mem::swap(&mut self.current_state, &mut old_state);

        let exchange_active = match self.ll.is_exchange_active() {
            Ok(state) => state,
            Err(_) => {
                self.ll.clear_go()?;
                return Ok(());
            }
        };

        let completed_exchange = if !exchange_active {
            None
        } else {
            match self.ll.complete_exchange() {
                Ok(amt) => {
                    defmt::info!("Arbitrator Completed! - {:?} bytes", amt);
                    Some(amt)
                },
                Err(Error::TransactionBusy) => None,
                Err(Error::TransactionAborted) => {
                    self.ll.clear_go()?;
                    return Ok(());
                }
                Err(_e) => {
                    defmt::error!("Exchange error! Aborting exchange");
                    self.ll.abort_exchange().ok();
                    self.ll.clear_go()?;
                    return Ok(());
                }
            }
        };

        if self.timeout_violated(&old_state) {
            defmt::warn!("Timeout violated!");
            if self.ll.is_exchange_active()? {
                defmt::warn!("Aborting exchange due to timeout");
                self.ll.abort_exchange().ok();
            }
            return Ok(());
        }

        self.current_state = match old_state {
            ArbState::Idle => {
                return Ok(());
            }
            ArbState::HeaderStart { t_window, .. } => {
                let amt_out = self
                    .outgoing_msgs
                    .cons
                    .read()
                    .map(|gr| gr.len())
                    .unwrap_or(0);
                self.smol_buf_out = (amt_out as u32).to_le_bytes();
                self.smol_buf_in = (0u32).to_le_bytes();

                self.ll.prepare_exchange(
                    self.smol_buf_out.as_ptr(),
                    4,
                    self.smol_buf_in.as_mut_ptr(),
                    4,
                )?;

                self.ll.notify_go()?;

                defmt::info!("Arbitrator: HeaderStart -> HeaderPrepped");

                ArbState::HeaderPrepped {
                    t_window,
                    t_step: self.timer.get_ticks(),
                }
            }
            ArbState::HeaderPrepped { t_window, t_step } => {
                if self.ll.has_exchange_begun()? {
                    defmt::info!("Arbitrator: HeaderPrepped -> HeaderXfer");
                    ArbState::HeaderXfer { t_window }
                } else {
                    ArbState::HeaderPrepped { t_window, t_step }
                }
            }
            ArbState::HeaderXfer { t_window } => {
                if let Some(amt) = completed_exchange {
                    if amt != 4 {
                        defmt::info!("Arbitrator: HeaderXfer -> Idle (BAD AMOUNT!)");

                        self.ll.clear_go()?;
                        return Ok(());
                    }

                    let amt_in = u32::from_le_bytes(self.smol_buf_in) as usize;
                    let amt_out = u32::from_le_bytes(self.smol_buf_out) as usize;

                    defmt::error!("Header in: {:?}, header out: {:?}", amt_in, amt_out);

                    if (amt_in == 0) && (amt_out == 0) {
                        self.ll.clear_go()?;
                        defmt::info!("Arbitrator: HeaderXfer -> Idle (Empty!)");
                        return Ok(());
                    }

                    if amt_in > 4096 {
                        defmt::error!("Illogical size!");
                        self.ll.clear_go()?;
                        return Ok(());
                    }

                    let out_ptr;
                    let out_len;
                    let in_ptr;
                    let in_len;

                    let wgr = if amt_in == 0 {
                        in_len = 0;
                        in_ptr = self.smol_buf_in.as_mut_ptr();
                        None
                    } else {
                        let mut wgr = self.incoming_msgs.prod.grant(amt_in)?;
                        in_len = amt_in;
                        in_ptr = wgr.as_mut_ptr();
                        Some(wgr)
                    };

                    let rgr = if let Some(rgr) = self.outgoing_msgs.cons.read() {
                        defmt::error!("Sending {:?}", &rgr[..]);
                        out_len = rgr.len();
                        out_ptr = rgr.as_ptr();
                        Some(rgr)
                    } else {
                        out_len = 0;
                        out_ptr = self.smol_buf_out.as_ptr();
                        None
                    };

                    self.ll.prepare_exchange(out_ptr, out_len, in_ptr, in_len)?;

                    defmt::info!("Arbitrator: HeaderXfer -> BodyPrepped");
                    ArbState::BodyPrepped {
                        t_window,
                        t_step: self.timer.get_ticks(),
                        fgr: rgr,
                        fgw: wgr,
                    }
                } else {
                    ArbState::HeaderXfer { t_window }
                }
            }
            ArbState::BodyPrepped {
                t_window,
                t_step,
                fgr,
                fgw,
            } => {
                if self.ll.has_exchange_begun()? {
                    defmt::info!("Arbitrator: BodyPrepped -> BodyXfer");
                    ArbState::BodyXfer { t_window, fgr, fgw }
                } else {
                    ArbState::BodyPrepped {
                        t_window,
                        t_step,
                        fgr,
                        fgw,
                    }
                }
            }
            ArbState::BodyXfer { t_window, fgr, fgw } => {
                if let Some(amt) = completed_exchange {
                    if let Some(gr) = fgr {
                        defmt::info!("Releasing message");
                        gr.release();
                    }

                    if let Some(gr) = fgw {
                        defmt::error!("Got {:?}", &gr[..amt]);
                        defmt::info!("Committing {:?} bytes", amt);
                        gr.commit(amt);
                    }

                    let now = self.timer.get_ticks();
                    defmt::info!("Arbitrator: BodyXfer -> HeaderStart");
                    ArbState::HeaderStart {
                        t_window: now,
                        t_step: now,
                    }
                } else {
                    ArbState::BodyXfer { t_window, fgr, fgw }
                }
            }
        };

        Ok(())
    }
}

impl<LL, CT, RT> ServerIoIn for EncLogicHLArbitrator<LL, CT, RT>
where
    CT: ArrayLength<u8>,
    LL: EncLogicLLArbitrator,
    RT: RollingTimer<Tick = u32>,
{
    fn recv<'a, 'b: 'a>(&'b mut self) -> core::result::Result<Option<Request<'b>>, ServerIoError> {
        self.current_grant = None;
        match self.dequeue() {
            Some(mut msg) => {
                msg.auto_release(true);
                self.current_grant = Some(msg);
                let sbr = self.current_grant.as_mut().unwrap();
                let len = sbr.len();

                defmt::trace!("Message contents: {:?}", &sbr[..]);

                // TODO: Cobs encoding at this level is probably not super necessary,
                // because for now we only handle one message exchange at a time. In the
                // future, it might be possible to pack multiple datagrams together into
                // a single frame. But for now, we only handle one.
                match from_bytes_cobs(sbr) {

                    Ok(deser) => {
                        defmt::info!("Giving Req!");
                        Ok(Some(Request {
                            source: self.uuid,
                            msg: deser,
                        }))
                    }
                    Err(_) => {
                        defmt::info!("Bad Req!");
                        if len == 0 {
                            defmt::info!("Bad Empty Req!");
                            Ok(None)
                        } else {
                            defmt::error!("Bad message on arbitrator deser");
                            Err(ServerIoError::DeserializeFailure)
                        }
                    }
                }
            }
            None => return Ok(None),
        }
    }
}
