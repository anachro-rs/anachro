/*

Thoughts:

* How to handle active pass/fail requests, like a pending subscription?
* Separate types for pre-registration?

*/

#![no_std]

use anachro_icd::{
    arbitrator::{
        Arbitrator, Control as AControl, ControlResponse, PubSubError, PubSubResponse,
    },
    component::{Component, ComponentInfo, Control as CControl, ControlType, PubSub, PubSubType, PubSubShort},
    Name, Uuid,
};
pub use anachro_icd::{self, Path, Version, PubSubPath, ManagedString};
pub use postcard::from_bytes;
pub use anachro_icd::arbitrator::SubMsg;
use serde::de::{Deserialize, DeserializeOwned};

pub struct Client {
    state: ClientState,
    name: Name<'static>,
    version: Version,
    ctr: u16,
    sub_paths: &'static [&'static str],
    pub_short_paths: &'static [&'static str],
    timeout_ticks: Option<u8>,
    uuid: Uuid,
}

pub enum ClientState {
    Created,
    PendingRegistration {
        ticks: u8,
    },
    Registered,
    Subscribing {
        ticks: u8,
        index: usize,
    },
    Subscribed,
    ShortCoding {
        ticks: u8,
        index: usize,
    },
    Active(ActiveState),
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
    fn as_active_mut(&mut self) -> Result<&mut ActiveState, Error> {
        match self {
            ClientState::Active(a_state) => Ok(a_state),
            _ => Err(Error::NotActive),
        }
    }

    fn as_active(&self) -> Result<&ActiveState, Error> {
        match self {
            ClientState::Active(a_state) => Ok(a_state),
            _ => Err(Error::NotActive),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ClientError {
    ParsingError,
}

pub trait ClientIo {
    fn recv(&mut self) -> Result<Option<Arbitrator>, ClientError>;
    fn send(&mut self, msg: &Component) -> Result<(), ClientError>;
}

pub struct ActiveState;

pub struct Deliverables<'a> {
    pub broker_response: Option<Component<'a>>,
    pub client_response: Option<SubMsg<'a>>,
}

pub struct RecvMsg<T: DeserializeOwned> {
    pub path: Path<'static>,
    pub payload: T
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
            state: ClientState::Created,
            sub_paths,
            pub_short_paths,
            timeout_ticks,
            uuid: Uuid::from_bytes([0u8; 16]),
        }
    }

    pub fn reset_connection(&mut self) {
        self.state = ClientState::Created;
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

    // pub fn subscribe<'a, 'b: 'a>(
    //     &mut self,
    //     path: PubSubPath<'a>,
    // ) -> Result<Component<'a>, Error> {
    //     // Only possible if we are already connected
    //     let state = self.state.as_active_mut()?;

    //     // TODO, we could track multiple pending subs in the
    //     // future, at the cost of storing a vec of pending subs
    //     if state.pending_sub {
    //         return Err(Error::Busy);
    //     }

    //     state.pending_sub = true;
    //     Ok(Component::PubSub(PubSub {
    //         path,
    //         ty: PubSubType::Sub
    //     }))
    // }

    pub fn publish<'a, 'b: 'a, C: ClientIo>(
        &'b self,
        cio: &mut C,
        path: &'a str,
        payload: &'a [u8]
    ) -> Result<(), Error> {
        self.state.as_active()?;

        let path = match self.pub_short_paths.iter().position(|pth| &path == pth) {
            Some(short) => PubSubPath::Short((short as u16) | 0x8000),
            None => PubSubPath::Long(ManagedString::Borrow(path))
        };

        let msg = Component::PubSub(PubSub {
            path,
            ty: PubSubType::Pub {
                payload
            }
        });

        cio.send(&msg)?;

        Ok(())
    }

    pub fn process_one<'a, 'b: 'a, C: ClientIo, T: DeserializeOwned>(
        &'b mut self,
        cio: &mut C,
    ) -> Result<Option<RecvMsg<T>>, Error> {

        let mut response = None;

        let msg = cio.recv()?;

        // TODO: split these into smaller functions
        let next = match (&mut self.state, msg) {
            // =====================================
            // Created
            // =====================================
            (ClientState::Created, _) => {
                self.ctr += 1;

                let resp = Component::Control(CControl {
                    seq: self.ctr,
                    ty: ControlType::RegisterComponent(ComponentInfo {
                        name: self.name.as_borrowed(),
                        version: self.version,
                    }),
                });

                cio.send(&resp)?;

                Some(ClientState::PendingRegistration { ticks: 0 })
            }
            // =====================================
            // Pending Registration
            // =====================================
            (ClientState::PendingRegistration { ref mut ticks }, Some(ref msg)) => {
                if let Arbitrator::Control(AControl { seq, response }) = msg {
                    if *seq != self.ctr {
                        // TODO, restart connection process? Just disregard?
                        return Err(Error::UnexpectedMessage);
                    }
                    if let Ok(ControlResponse::ComponentRegistration(uuid)) = response {
                        self.uuid = *uuid;

                        Some(ClientState::Registered)
                    } else {
                        // TODO, restart connection process? Just disregard?
                        return Err(Error::UnexpectedMessage);
                    }
                } else {
                    *ticks = ticks.saturating_add(1);
                    None
                }
            }
            (ClientState::PendingRegistration { ref mut ticks }, None) => {
                *ticks = ticks.saturating_add(1);
                // TODO: Some kind of timeout? Just wait forever?
                None
            }

            // =====================================
            // Registered
            // =====================================
            (ClientState::Registered, _) => {
                if self.sub_paths.is_empty() {
                    Some(ClientState::Subscribed)
                } else {
                    let msg = Component::PubSub(PubSub {
                        path: PubSubPath::Long(Path::borrow_from_str(self.sub_paths[0])),
                        ty: PubSubType::Sub
                    });

                    cio.send(&msg)?;

                    Some(ClientState::Subscribing {
                        ticks: 0,
                        index: 0,
                    })
                }
            }

            // =====================================
            // Subscribing
            // =====================================
            (ClientState::Subscribing { ref mut ticks, .. }, None) => {
                *ticks = ticks.saturating_add(1);
                None
            }

            (ClientState::Subscribing { ref mut ticks, ref mut index }, Some(ref msg)) => {
                if let Arbitrator::PubSub(Ok(PubSubResponse::SubAck { path: PubSubPath::Long(pth) })) = msg {
                    if pth.as_str() == self.sub_paths[*index] {
                        *index += 1;
                        if *index >= self.sub_paths.len() {
                            Some(ClientState::Subscribed)
                        } else {
                            let msg = Component::PubSub(PubSub {
                                path: PubSubPath::Long(Path::borrow_from_str(self.sub_paths[*index])),
                                ty: PubSubType::Sub
                            });

                            cio.send(&msg)?;

                            Some(ClientState::Subscribing {
                                ticks: 0,
                                index: *index,
                            })
                        }
                    } else {
                        *ticks = ticks.saturating_add(1);
                        None
                    }
                } else {
                    *ticks = ticks.saturating_add(1);
                    None
                }
            }

            // =====================================
            // Subscribed
            // =====================================
            (ClientState::Subscribed, _) => {
                match (self.sub_paths.len(), self.pub_short_paths.len()) {
                    (0, 0) => Some(ClientState::Active(ActiveState)),
                    (0, _n) => {
                        self.ctr = self.ctr.wrapping_add(1);
                        let msg = Component::Control(CControl {
                            seq: self.ctr,
                            ty: ControlType::RegisterPubSubShortId(PubSubShort {
                                long_name: self.pub_short_paths[0],
                                short_id: 0x8000,
                            })
                        });

                        cio.send(&msg)?;

                        Some(ClientState::ShortCoding {
                            ticks: 0,
                            index: 0x8000,
                        })
                    }
                    (n, _) => {
                        self.ctr = self.ctr.wrapping_add(1);
                        let msg = Component::Control(CControl {
                            seq: self.ctr,
                            ty: ControlType::RegisterPubSubShortId(PubSubShort {
                                long_name: self.sub_paths[0],
                                short_id: 0x0000,
                            })
                        });

                        cio.send(&msg)?;

                        Some(ClientState::ShortCoding {
                            ticks: 0,
                            index: 0,
                        })
                    }
                }
            }

            // =====================================
            // ShortCoding
            // =====================================
            (ClientState::ShortCoding { ref mut ticks, .. }, None) => {
                *ticks = ticks.saturating_add(1);
                None
            }

            (ClientState::ShortCoding { ref mut ticks, ref mut index }, Some(ref msg)) => {
                if let Arbitrator::Control(AControl { seq, response: Ok(ControlResponse::PubSubShortRegistration(sid)) }) = msg {
                    if *seq == self.ctr && *sid == (*index as u16) {
                        if *index < 0x8000 {
                            todo!("Sub to some more sub paths, or start pub shorts")
                        } else {
                            todo!("sub to pub shorts, or move to active")
                        }
                    } else {
                        *ticks = ticks.saturating_add(1);
                        None
                    }
                } else {
                    *ticks = ticks.saturating_add(1);
                    None
                }
            }

            // =====================================
            // Active
            // =====================================
            (ClientState::Active(ref mut a_state), Some(Arbitrator::Control(ref ctl)))
                if ctl.seq == self.ctr =>
            {
                // TODO: Can this generate any kind of response to the user or broker? Update state?
                a_state.process_control(ctl)?;
                None
            }
            (ClientState::Active(ref mut a_state), Some(Arbitrator::PubSub(ref ps))) => {
                let pubsub = a_state.process_pubsub(ps)?;

                response = match pubsub {
                    Some(ps) => {
                        // Determine the path
                        let path = match ps.path {
                            PubSubPath::Short(sid) => {
                                Path::Borrow(*self.sub_paths.get(sid as usize).ok_or(Error::UnexpectedMessage)?)
                            }
                            PubSubPath::Long(ms) => {
                                ms.try_to_owned().unwrap()
                            }
                        };

                        Some(RecvMsg {
                            path,
                            payload: from_bytes(ps.payload).map_err(|_| Error::UnexpectedMessage)?,
                        })
                    }
                    None => None,
                };

                None
            }
            (ClientState::Active(_), Some(_)) => {
                // TODO: Process any other kind of non-pubsub Arbitrator message?
                None
            }
            (ClientState::Active(_), None) => {
                // Todo: any periodic keepalive pings?
                None
            }
        };

        if let Some(new_state) = next {
            self.state = new_state;
        }

        Ok(response)
    }
}

impl ActiveState {
    fn process_control(&mut self, msg: &AControl) -> Result<(), Error> {
        // match msg.response {
        //     Ok(ControlResponse::ComponentRegistration(_)) => {
        //         // We already registered?
        //         return Err(Error::UnexpectedMessage);
        //     }
        //     Ok(ControlResponse::PubSubShortRegistration(short_id)) => {
        //         if let Some(exp_id) = self.pending_short {
        //             if exp_id != short_id {
        //                 // This wasn't the shortcode response we were expecting
        //                 return Err(Error::UnexpectedMessage);
        //             }
        //         } else {
        //             // We weren't expecting a shortcode response?
        //             return Err(Error::UnexpectedMessage);
        //         }
        //         // We got what we were expecting! Clear it
        //         self.pending_short = None;
        //     }
        //     Err(_) => {
        //         // ?
        //         return Err(Error::UnexpectedMessage);
        //     }
        // }
        // Ok(())
        todo!()
    }

    fn process_pubsub<'a>(
        &mut self,
        msg: &'a Result<PubSubResponse, PubSubError>,
    ) -> Result<Option<SubMsg<'a>>, Error> {
        // match msg {
        //     Ok(PubSubResponse::SubAck { .. }) => {
        //         if self.pending_sub {
        //             // TODO: Check we're getting the right subscription?
        //             self.pending_sub = false;
        //             Ok(None)
        //         } else {
        //             Err(Error::UnexpectedMessage)
        //         }
        //     }
        //     Ok(PubSubResponse::SubMsg(SubMsg { path, payload })) => Ok(Some(SubMsg {
        //         path: path.clone(),
        //         payload,
        //     })),
        //     Err(_) => Err(Error::UnexpectedMessage),
        // }
        todo!()
    }
}

pub enum TableError {
    NoMatch,
    Postcard(postcard::Error),
    SorryNoShortCodes,
}

// TODO: Postcard feature?

/// ## Example
/// table_recv!(
///     PlantLightTable,
///     Relay: "lights/plants/living-room" => RelayCommand,
///     Time: "time/unix/local" => u32,
/// );
#[macro_export]
macro_rules! table_recv {
    ($enum_ty:ident, $($variant_name:ident: $path:expr => $variant_ty:ty,)+) => {
        #[derive(Debug)]
        pub enum $enum_ty {
            $($variant_name($variant_ty)),+
        }

        impl $enum_ty {
            pub fn from_pub_sub<'a>(msg: $crate::SubMsg<'a>) -> core::result::Result<Self, $crate::TableError> {
                let msg_path = match msg.path {
                    $crate::anachro_icd::PubSubPath::Long(path) => path,
                    _ => return Err($crate::TableError::SorryNoShortCodes),
                };
                $(
                    if matches(msg_path.as_str(), $path) {
                        return Ok(
                            $enum_ty::$variant_name(
                                $crate::postcard::from_bytes(msg.payload)
                                    .map_err(|e| $crate::TableError::Postcard(e))?
                            )
                        );
                    }
                )+
                Err($crate::TableError::NoMatch)
            }

            pub const fn paths() -> &'static [&'static str] {
                const PATHS: &[&str] = &[
                    $($path,)+
                ];

                PATHS
            }
        }
    };
}
