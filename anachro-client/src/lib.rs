/*

Thoughts:

* How to handle active pass/fail requests, like a pending subscription?
* Separate types for pre-registration?

*/

#![no_std]

pub use anachro_icd::arbitrator::SubMsg;
pub use anachro_icd::{self, ManagedString, Path, PubSubPath, Version};
use anachro_icd::{
    arbitrator::{Arbitrator, Control as AControl, ControlResponse, PubSubResponse},
    component::{
        Component, ComponentInfo, Control as CControl, ControlType, PubSub, PubSubShort, PubSubType,
    },
    Name, Uuid,
};
pub use postcard::from_bytes;

pub struct Client {
    state: ClientState,
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

#[derive(Debug)]
pub enum ClientState {
    Disconnected,
    PendingRegistration,
    Registered,
    Subscribing,
    Subscribed,
    ShortCodingSub,
    ShortCodingPub,
    Active,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    NotActive,
    Busy,
    UnexpectedMessage,
    ClientIoError(ClientError),
}

impl From<ClientError> for Error {
    fn from(other: ClientError) -> Self {
        Error::ClientIoError(other)
    }
}

impl ClientState {
    fn as_active(&self) -> Result<(), Error> {
        match self {
            ClientState::Active => Ok(()),
            _ => Err(Error::NotActive),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ClientError {
    ParsingError,
    NoData,
    OutputFull,
}

pub trait ClientIo {
    fn recv(&mut self) -> Result<Option<Arbitrator>, ClientError>;
    fn send(&mut self, msg: &Component) -> Result<(), ClientError>;
}

pub struct ActiveState;

#[derive(Debug)]
pub struct RecvMsg<T: Table> {
    pub path: Path<'static>,
    pub payload: T,
}

impl Client {
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

    pub fn reset_connection(&mut self) {
        self.state = ClientState::Disconnected;
        self.current_tick = 0;
        self.current_idx = 0;
    }

    pub fn get_id(&self) -> Option<&Uuid> {
        if self.is_connected() {
            Some(&self.uuid)
        } else {
            None
        }
    }

    pub fn is_connected(&self) -> bool {
        self.state.as_active().is_ok()
    }

    pub fn publish<'a, 'b: 'a, C: ClientIo>(
        &'b self,
        cio: &mut C,
        path: &'a str,
        payload: &'a [u8],
    ) -> Result<(), Error> {
        self.state.as_active()?;

        let path = match self.pub_short_paths.iter().position(|pth| &path == pth) {
            Some(short) => PubSubPath::Short((short as u16) | 0x8000),
            None => PubSubPath::Long(ManagedString::Borrow(path)),
        };

        let msg = Component::PubSub(PubSub {
            path,
            ty: PubSubType::Pub { payload },
        });

        cio.send(&msg)?;

        Ok(())
    }

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

    fn pending_registration<C: ClientIo>(&mut self, cio: &mut C) -> Result<(), Error> {
        let msg = cio.recv()?;
        let msg = match msg {
            Some(msg) => msg,
            None => {
                self.current_tick = self.current_tick.saturating_add(1);
                return Ok(());
            }
        };

        if let Arbitrator::Control(AControl { seq, response }) = msg {
            if seq != self.ctr {
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
                // TODO, restart connection process? Just disregard?
                Err(Error::UnexpectedMessage)
            }
        } else {
            self.current_tick = self.current_tick.saturating_add(1);
            Ok(())
        }
    }

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
                        short_id: 0x8000,
                    }),
                });

                cio.send(&msg)?;

                self.state = ClientState::ShortCodingPub;
                self.current_tick = 0;
                self.current_idx = 0;
            }
            (_n, _) => {
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
                                short_id: 0x8000,
                            }),
                        });

                        cio.send(&msg)?;

                        self.current_tick = 0;
                        self.current_idx = 0;
                        self.state = ClientState::ShortCodingPub;
                    }
                } else {
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
            } else {
                self.current_tick = self.current_tick.saturating_add(1);
            }
        } else {
            self.current_tick = self.current_tick.saturating_add(1);
        }

        Ok(())
    }

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
            if seq == self.ctr && sid == ((self.current_idx + 0x8000) as u16) {
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
                            short_id: (self.current_idx + 0x8000) as u16,
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

    pub fn active<C: ClientIo, T: Table>(
        &mut self,
        cio: &mut C,
    ) -> Result<Option<RecvMsg<T>>, Error> {
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

    fn timeout_violated(&self) -> bool {
        match self.timeout_ticks {
            Some(ticks) if ticks <= self.current_tick => true,
            Some(_) => false,
            None => false,
        }
    }

    pub fn process_one<C: ClientIo, T: Table>(
        &mut self,
        cio: &mut C,
    ) -> Result<Option<RecvMsg<T>>, Error> {
        let mut response: Option<RecvMsg<T>> = None;

        // TODO: split these into smaller functions
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
                            short_id: (self.current_idx + 0x8000) as u16,
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

pub enum TableError {
    NoMatch,
    Postcard(postcard::Error),
    SorryNoShortCodes,
}

pub trait Table: Sized {
    fn sub_paths() -> &'static [&'static str];
    fn pub_paths() -> &'static [&'static str];
    fn from_pub_sub<'a>(msg: &'a SubMsg<'a>) -> Result<Self, TableError>;
}

pub struct Pubby<'a> {
    pub buf: &'a [u8],
    pub path: &'static str,
}

// TODO: Postcard feature?

/// ## Example
/// pubsub_table!{
///     AnachroTable,
///     Subs => {
///         Something: "foo/bar/baz" => Demo,
///         Else: "bib/bim/bap" => (),
///     },
///     Pubs => {
///         Etwas: "short/send" => (),
///         Anders: "send/short" => (),
///     },
/// }
#[macro_export]
macro_rules! pubsub_table {
    (
        $enum_ty:ident,
        Subs => {
            $($sub_variant_name:ident: $sub_path:expr => $sub_variant_ty:ty,)+
        },
        Pubs => {
            $($pub_variant_name:ident: $pub_path:expr => $pub_variant_ty:ty,)+
        },
    ) => {
        #[derive(Debug, serde::Deserialize, Clone)]
        pub enum $enum_ty {
            $($sub_variant_name($sub_variant_ty)),+,
            $($pub_variant_name($pub_variant_ty)),+,
        }

        impl $crate::Table for $enum_ty {
            fn from_pub_sub<'a>(msg: &'a $crate::SubMsg<'a>) -> core::result::Result<Self, $crate::TableError> {
                let msg_path = match msg.path {
                    $crate::anachro_icd::PubSubPath::Long(ref path) => path.as_str(),
                    $crate::anachro_icd::PubSubPath::Short(sid) => {
                        if sid < 0x8000 {
                            // Subscribe
                            if (sid as usize) < Self::sub_paths().len() {
                                Self::sub_paths()[(sid as usize)]
                            } else {
                                return Err($crate::TableError::NoMatch);
                            }
                        } else {
                            // publish
                            let new_sid = (sid as usize) - 0x8000;
                            if new_sid < Self::pub_paths().len() {
                                Self::pub_paths()[new_sid]
                            } else {
                                return Err($crate::TableError::NoMatch);
                            }
                        }
                    },
                };
                $(
                    if $crate::anachro_icd::matches(msg_path, $sub_path) {
                        return Ok(
                            $enum_ty::$sub_variant_name(
                                postcard::from_bytes(msg.payload)
                                    .map_err(|e| $crate::TableError::Postcard(e))?
                            )
                        );
                    }
                )+
                $(
                    if $crate::anachro_icd::matches(msg_path, $pub_path) {
                        return Ok(
                            $enum_ty::$pub_variant_name(
                                postcard::from_bytes(msg.payload)
                                    .map_err(|e| $crate::TableError::Postcard(e))?
                            )
                        );
                    }
                )+
                Err($crate::TableError::NoMatch)
            }

            fn sub_paths() -> &'static [&'static str] {
                Self::sub_paths()
            }

            fn pub_paths() -> &'static [&'static str] {
                Self::pub_paths()
            }
        }

        impl $enum_ty {
            pub fn get_pub_path(&self) -> core::option::Option<&'static str> {
                match self {
                    $(
                        $enum_ty::$pub_variant_name(_) => Some($pub_path),
                    )+
                    _ => None,
                }
            }

            pub fn serialize<'a>(&self, buffer: &'a mut [u8]) -> core::result::Result<$crate::Pubby<'a>, ()> {
                match self {
                    $(
                        $enum_ty::$pub_variant_name(msg) => {
                            Ok($crate::Pubby {
                                buf: postcard::to_slice(msg, buffer)
                                        .map_err(drop)?,
                                path: $pub_path,
                            })
                        },
                    )+
                    _ => Err(()),
                }
            }

            pub const fn sub_paths() -> &'static [&'static str] {
                const PATHS: &[&str] = &[
                    $($sub_path,)+
                ];

                PATHS
            }

            pub const fn pub_paths() -> &'static [&'static str] {
                const PATHS: &[&str] = &[
                    $($pub_path,)+
                ];

                PATHS
            }
        }
    };
}
