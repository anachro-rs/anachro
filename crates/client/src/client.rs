//! The Client interface
//!
//! This is the primary interface used by clients. It is used to track
//! the state of a connection, and process any incoming or outgoing messages

use {
    crate::{client_io::ClientIo, table::Table, Error, RecvMsg},
    anachro_icd::{
        self,
        arbitrator::{Arbitrator, Control as AControl, ControlResponse, PubSubResponse},
        component::{
            Component, ComponentInfo, Control as CControl, ControlType, PubSub, PubSubShort,
            PubSubType,
        },
        ManagedString, Name, Path, PubSubPath, Uuid, Version,
    },
};

/// The shortcode offset used for Publish topics
///
/// For now, I have defined `0x0000..=0x7FFF` as the range used
/// for subscription topic shortcodes, and `0x8000..=0xFFFF` as
/// the range used for publish topic shortcodes. This is an
/// implementation detail, and should not be relied upon.
pub const PUBLISH_SHORTCODE_OFFSET: u16 = 0x8000;

#[derive(Debug)]
enum ClientState {
    Disconnected,
    PendingRegistration,
    Registered,
    Subscribing,
    Subscribed,
    ShortCodingSub,
    ShortCodingPub,
    Active,
}

impl ClientState {
    pub(crate) fn as_active(&self) -> Result<(), Error> {
        match self {
            ClientState::Active => Ok(()),
            _ => Err(Error::NotActive),
        }
    }
}

/// The Client interface
///
/// This is the primary interface used by clients. It is used to track
/// the state of a connection, and process any incoming or outgoing messages
pub struct Client {
    state: ClientState,
    // TODO: This should probably just be a &'static str
    name: Name<'static>,
    version: Version,
    ctr: u16,
    sub_paths: &'static [&'static str],
    pub_short_paths: &'static [&'static str],
    timeout_ticks: Option<u8>,
    uuid: Uuid,
    current_tick: u8,
    current_idx: usize,
}

impl Client {
    /// Create a new client instance
    ///
    /// ## Parameters
    ///
    /// ### `name`
    ///
    /// The name of this device or client
    ///
    /// ### `version`
    ///
    /// The semantic version number of this client
    ///
    /// ### `ctr_init`
    ///
    /// A value to initialize the control counter.
    ///
    /// You may choose to initialize this with a fixed or random value
    ///
    /// ### `sub_paths`
    ///
    /// The subscription paths that the device is interested in
    ///
    /// This is typically provided by the `Table::sub_paths()` method on
    /// a table type generated using the `pubsub_table!()` macro.
    ///
    /// ### `pub_paths`
    ///
    /// The publishing paths that the device is interested in
    ///
    /// This is typically provided by the `Table::pub_paths()` method on
    /// a table type generated using the `pubsub_table!()` macro.
    ///
    /// ### `timeout_ticks`
    ///
    /// The number of ticks used to time-out waiting for certain responses
    /// before retrying automatically.
    ///
    /// Set to `None` to disable automatic retries. This will require the
    /// user to manually call `Client::reset_connection()` if a message is
    /// lost.
    ///
    /// Ticks are counted by calls to `Client::process_one()`, which should be
    /// called at a semi-regular rate. e.g. if you call `process_one()` every
    /// 10ms, a `timeout_ticks: Some(100)` would automatically timeout after
    /// 1s of waiting for a response.
    pub fn new(
        name: &str,
        version: Version,
        ctr_init: u16,
        sub_paths: &'static [&'static str],
        pub_short_paths: &'static [&'static str],
        timeout_ticks: Option<u8>,
    ) -> Self {
        Self {
            name: Name::try_from_str(name).unwrap(),
            version,
            ctr: ctr_init,
            state: ClientState::Disconnected,
            sub_paths,
            pub_short_paths,
            timeout_ticks,
            uuid: Uuid::from_bytes([0u8; 16]),
            current_tick: 0,
            current_idx: 0,
        }
    }

    /// Reset the client connection
    ///
    /// This immediately disconnects the client, at which point
    /// it will begin attemption to re-establish a connection to
    /// the broker.
    pub fn reset_connection(&mut self) {
        self.state = ClientState::Disconnected;
        self.current_tick = 0;
        self.current_idx = 0;
    }

    /// Obtain the `Uuid` assigned by the broker to this client
    ///
    /// If the client is not connected, `None` will be returned.
    pub fn get_id(&self) -> Option<&Uuid> {
        if self.is_connected() {
            Some(&self.uuid)
        } else {
            None
        }
    }

    /// Is the client connected?
    pub fn is_connected(&self) -> bool {
        self.state.as_active().is_ok()
    }

    /// Publish a message
    ///
    /// This interface publishes a message from the client to the broker.
    ///
    /// ## Parameters
    ///
    /// ### `cio`
    ///
    /// This is the `ClientIo` instance used by the client
    ///
    /// ### `path`
    ///
    /// This is the path to publish to. This is typically created by using
    /// the `Table::serialize()` method, which returns a path and the serialized
    /// payload
    ///
    /// ### `payload`
    ///
    /// The serialized payload to publish. This is typically created by using
    /// the `Table::serialize()` method, which returns a path and the serialized
    /// payload
    pub fn publish<'a, 'b: 'a, C: ClientIo>(
        &'b self,
        cio: &mut C,
        path: &'a str,
        payload: &'a [u8],
    ) -> Result<(), Error> {
        self.state.as_active()?;

        let path = match self.pub_short_paths.iter().position(|pth| &path == pth) {
            Some(short) => PubSubPath::Short((short as u16) | PUBLISH_SHORTCODE_OFFSET),
            None => PubSubPath::Long(ManagedString::Borrow(path)),
        };

        let msg = Component::PubSub(PubSub {
            path,
            ty: PubSubType::Pub { payload },
        });

        cio.send(&msg)?;

        Ok(())
    }

    /// Process a single incoming message
    ///
    /// This function *must* be called regularly to process messages
    /// that have been received by the broker. It is suggested to call
    /// it at regular intervals if you are using the `timeout_ticks` option
    /// when creating the Client.
    ///
    /// If a subscription message has been received, it will be returned
    /// with the path that was published to. If the Client has subscribed
    /// using a wildcard, it may not exactly match the subscription topic
    ///
    /// The `anachro-icd::matches` function can be used to compare if a topic
    /// matches a given fixed or wildcard path, if necessary.
    pub fn process_one<C: ClientIo, T: Table>(
        &mut self,
        cio: &mut C,
    ) -> Result<Option<RecvMsg<T>>, Error> {
        let mut response: Option<RecvMsg<T>> = None;

        match &mut self.state {
            // =====================================
            // Disconnected
            // =====================================
            ClientState::Disconnected => {
                self.disconnected(cio)?;
            }

            // =====================================
            // Pending Registration
            // =====================================
            ClientState::PendingRegistration => {
                self.pending_registration(cio)?;

                if self.timeout_violated() {
                    // println!("violated!");
                    self.state = ClientState::Disconnected;
                    self.current_tick = 0;
                }
            }

            // =====================================
            // Registered
            // =====================================
            ClientState::Registered => {
                self.registered(cio)?;
            }

            // =====================================
            // Subscribing
            // =====================================
            ClientState::Subscribing => {
                self.subscribing(cio)?;

                if self.timeout_violated() {
                    let msg = Component::PubSub(PubSub {
                        path: PubSubPath::Long(Path::borrow_from_str(
                            self.sub_paths[self.current_idx],
                        )),
                        ty: PubSubType::Sub,
                    });

                    cio.send(&msg)?;

                    self.current_tick = 0;
                }
            }

            // =====================================
            // Subscribed
            // =====================================
            ClientState::Subscribed => {
                self.subscribed(cio)?;
            }

            // =====================================
            // ShortCoding
            // =====================================
            ClientState::ShortCodingSub => {
                self.shortcoding_sub(cio)?;

                if self.timeout_violated() {
                    self.ctr = self.ctr.wrapping_add(1);

                    let msg = Component::Control(CControl {
                        seq: self.ctr,
                        ty: ControlType::RegisterPubSubShortId(PubSubShort {
                            long_name: self.sub_paths[self.current_idx],
                            short_id: self.current_idx as u16,
                        }),
                    });

                    cio.send(&msg)?;

                    self.current_tick = 0;
                }
            }

            ClientState::ShortCodingPub => {
                self.shortcoding_pub(cio)?;

                if self.timeout_violated() {
                    self.ctr = self.ctr.wrapping_add(1);

                    let msg = Component::Control(CControl {
                        seq: self.ctr,
                        ty: ControlType::RegisterPubSubShortId(PubSubShort {
                            long_name: self.pub_short_paths[self.current_idx],
                            short_id: (self.current_idx as u16) | PUBLISH_SHORTCODE_OFFSET,
                        }),
                    });

                    cio.send(&msg)?;

                    self.current_tick = 0;
                }
            }

            // =====================================
            // Active
            // =====================================
            ClientState::Active => {
                response = self.active(cio)?;
            }
        };

        Ok(response)
    }
}

// Private interfaces for the client. These are largely used to
// process incoming messages and handle state
impl Client {
    /// Have we reached the timeout limit provided by the user?
    fn timeout_violated(&self) -> bool {
        match self.timeout_ticks {
            Some(ticks) if ticks <= self.current_tick => true,
            Some(_) => false,
            None => false,
        }
    }

    /// Process messages while in a `ClientState::Disconnected` state
    fn disconnected<C: ClientIo>(&mut self, cio: &mut C) -> Result<(), Error> {
        self.ctr += 1;

        let resp = Component::Control(CControl {
            seq: self.ctr,
            ty: ControlType::RegisterComponent(ComponentInfo {
                name: self.name.as_borrowed(),
                version: self.version,
            }),
        });

        cio.send(&resp)?;

        self.state = ClientState::PendingRegistration;
        self.current_tick = 0;

        Ok(())
    }

    /// Process messages while in a `ClientState::PendingRegistration state`
    fn pending_registration<C: ClientIo>(&mut self, cio: &mut C) -> Result<(), Error> {
        let msg = cio.recv()?;
        let msg = match msg {
            Some(msg) => msg,
            None => {
                self.current_tick = self.current_tick.saturating_add(1);
                return Ok(());
            }
        };

        // println!("got pr mesg!");


        if let Arbitrator::Control(AControl { seq, response }) = msg {
            if seq != self.ctr {
                // println!("ctr mismatch! {} {}", seq, self.ctr);
                self.current_tick = self.current_tick.saturating_add(1);
                // TODO, restart connection process? Just disregard?
                Err(Error::UnexpectedMessage)
            } else if let Ok(ControlResponse::ComponentRegistration(uuid)) = response {
                self.uuid = uuid;
                self.state = ClientState::Registered;
                self.current_tick = 0;
                Ok(())
            } else {
                self.current_tick = self.current_tick.saturating_add(1);
                // println!("Other Error");
                // TODO, restart connection process? Just disregard?
                Err(Error::UnexpectedMessage)
            }
        } else {
            // println!("??? {:?}?", msg);
            self.current_tick = self.current_tick.saturating_add(1);
            Ok(())
        }
    }

    /// Process messages while in a `ClientState::Registered` state
    fn registered<C: ClientIo>(&mut self, cio: &mut C) -> Result<(), Error> {
        if self.sub_paths.is_empty() {
            self.state = ClientState::Subscribed;
            self.current_tick = 0;
        } else {
            let msg = Component::PubSub(PubSub {
                path: PubSubPath::Long(Path::borrow_from_str(self.sub_paths[0])),
                ty: PubSubType::Sub,
            });

            cio.send(&msg)?;

            self.state = ClientState::Subscribing;
            self.current_idx = 0;
            self.current_tick = 0;
        }

        Ok(())
    }

    /// Process messages while in a `ClientState::Subscribing` state
    fn subscribing<C: ClientIo>(&mut self, cio: &mut C) -> Result<(), Error> {
        let msg = cio.recv()?;
        let msg = match msg {
            Some(msg) => msg,
            None => {
                self.current_tick = self.current_tick.saturating_add(1);
                return Ok(());
            }
        };

        if let Arbitrator::PubSub(Ok(PubSubResponse::SubAck {
            path: PubSubPath::Long(pth),
        })) = msg
        {
            if pth.as_str() == self.sub_paths[self.current_idx] {
                self.current_idx += 1;
                if self.current_idx >= self.sub_paths.len() {
                    self.state = ClientState::Subscribed;
                    self.current_tick = 0;
                } else {
                    let msg = Component::PubSub(PubSub {
                        path: PubSubPath::Long(Path::borrow_from_str(
                            self.sub_paths[self.current_idx],
                        )),
                        ty: PubSubType::Sub,
                    });

                    cio.send(&msg)?;

                    self.state = ClientState::Subscribing;
                    self.current_tick = 0;
                }
            } else {
                self.current_tick = self.current_tick.saturating_add(1);
            }
        } else {
            self.current_tick = self.current_tick.saturating_add(1);
        }

        Ok(())
    }

    /// Process messages while in a `ClientState::Subscribed` state
    fn subscribed<C: ClientIo>(&mut self, cio: &mut C) -> Result<(), Error> {
        match (self.sub_paths.len(), self.pub_short_paths.len()) {
            (0, 0) => {
                self.state = ClientState::Active;
                self.current_tick = 0;
            }
            (0, _n) => {
                self.ctr = self.ctr.wrapping_add(1);
                let msg = Component::Control(CControl {
                    seq: self.ctr,
                    ty: ControlType::RegisterPubSubShortId(PubSubShort {
                        long_name: self.pub_short_paths[0],
                        short_id: PUBLISH_SHORTCODE_OFFSET,
                    }),
                });

                cio.send(&msg)?;

                self.state = ClientState::ShortCodingPub;
                self.current_tick = 0;
                self.current_idx = 0;
            }
            (_n, _) => {
                // TODO: This doesn't handle the case when the subscribe shortcode is
                // a wildcard, which the broker will reject
                self.ctr = self.ctr.wrapping_add(1);
                let msg = Component::Control(CControl {
                    seq: self.ctr,
                    ty: ControlType::RegisterPubSubShortId(PubSubShort {
                        long_name: self.sub_paths[0],
                        short_id: 0x0000,
                    }),
                });

                cio.send(&msg)?;

                self.state = ClientState::ShortCodingSub;
                self.current_tick = 0;
                self.current_idx = 0;
            }
        }
        Ok(())
    }

    /// Process messages while in a `ClientState::ShortcodingSub` state
    fn shortcoding_sub<C: ClientIo>(&mut self, cio: &mut C) -> Result<(), Error> {
        let msg = cio.recv()?;
        let msg = match msg {
            Some(msg) => msg,
            None => {
                self.current_tick = self.current_tick.saturating_add(1);
                return Ok(());
            }
        };

        if let Arbitrator::Control(AControl {
            seq,
            response: Ok(ControlResponse::PubSubShortRegistration(sid)),
        }) = msg
        {
            if seq == self.ctr && sid == (self.current_idx as u16) {
                self.current_idx += 1;

                if self.current_idx >= self.sub_paths.len() {
                    if self.pub_short_paths.is_empty() {
                        self.state = ClientState::Active;
                        self.current_tick = 0;
                    } else {
                        self.ctr = self.ctr.wrapping_add(1);

                        let msg = Component::Control(CControl {
                            seq: self.ctr,
                            ty: ControlType::RegisterPubSubShortId(PubSubShort {
                                long_name: self.pub_short_paths[0],
                                short_id: PUBLISH_SHORTCODE_OFFSET,
                            }),
                        });

                        cio.send(&msg)?;

                        self.current_tick = 0;
                        self.current_idx = 0;
                        self.state = ClientState::ShortCodingPub;
                    }
                } else {
                    self.ctr = self.ctr.wrapping_add(1);

                    // TODO: This doesn't handle subscriptions with wildcards
                    let msg = Component::Control(CControl {
                        seq: self.ctr,
                        ty: ControlType::RegisterPubSubShortId(PubSubShort {
                            long_name: self.sub_paths[self.current_idx],
                            short_id: self.current_idx as u16,
                        }),
                    });

                    cio.send(&msg)?;

                    self.current_tick = 0;
                }
            } else {
                self.current_tick = self.current_tick.saturating_add(1);
            }
        } else {
            self.current_tick = self.current_tick.saturating_add(1);
        }

        Ok(())
    }

    /// Process messages while in a `ClientState::ShortcodingPub` state
    fn shortcoding_pub<C: ClientIo>(&mut self, cio: &mut C) -> Result<(), Error> {
        let msg = cio.recv()?;
        let msg = match msg {
            Some(msg) => msg,
            None => {
                self.current_tick = self.current_tick.saturating_add(1);
                return Ok(());
            }
        };

        if let Arbitrator::Control(AControl {
            seq,
            response: Ok(ControlResponse::PubSubShortRegistration(sid)),
        }) = msg
        {
            if seq == self.ctr && sid == ((self.current_idx as u16) | PUBLISH_SHORTCODE_OFFSET) {
                self.current_idx += 1;

                if self.current_idx >= self.pub_short_paths.len() {
                    self.state = ClientState::Active;
                    self.current_tick = 0;
                } else {
                    self.ctr = self.ctr.wrapping_add(1);

                    let msg = Component::Control(CControl {
                        seq: self.ctr,
                        ty: ControlType::RegisterPubSubShortId(PubSubShort {
                            long_name: self.pub_short_paths[self.current_idx],
                            short_id: ((self.current_idx as u16) | PUBLISH_SHORTCODE_OFFSET),
                        }),
                    });

                    cio.send(&msg)?;

                    self.current_tick = 0;
                }
            } else {
                self.current_tick = self.current_tick.saturating_add(1);
            }
        } else {
            self.current_tick = self.current_tick.saturating_add(1);
        }

        Ok(())
    }

    /// Process messages while in a Connected state
    fn active<C: ClientIo, T: Table>(&mut self, cio: &mut C) -> Result<Option<RecvMsg<T>>, Error> {
        let msg = cio.recv()?;
        let pubsub = match msg {
            Some(Arbitrator::PubSub(Ok(PubSubResponse::SubMsg(ref ps)))) => ps,
            Some(_) => {
                // TODO: Maybe something else? return err?
                return Ok(None);
            }
            None => {
                return Ok(None);
            }
        };

        // Determine the path
        let path = match &pubsub.path {
            PubSubPath::Short(sid) => Path::Borrow(
                *self
                    .sub_paths
                    .get(*sid as usize)
                    .ok_or(Error::UnexpectedMessage)?,
            ),
            PubSubPath::Long(ms) => ms.try_to_owned().map_err(|_| Error::UnexpectedMessage)?,
        };

        Ok(Some(RecvMsg {
            path,
            payload: T::from_pub_sub(pubsub).map_err(|_| Error::UnexpectedMessage)?,
        }))
    }
}
