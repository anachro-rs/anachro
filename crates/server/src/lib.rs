//! # The Anachro Protocol Server/Broker Library
//!
//! This crate is used by devices acting as a Server/Broker of the Anachro Protocol

#![no_std]

use {
    anachro_icd::{
        arbitrator::{self, Arbitrator, Control as AControl, ControlError, SubMsg},
        component::{
            Component, ComponentInfo, Control, ControlType, PubSub, PubSubShort, PubSubType,
        },
        ManagedString,
    },
    core::default::Default,
    heapless::{consts, Vec},
};

pub use anachro_icd::{self, Name, Path, PubSubPath, Uuid, Version};

type ClientStore = Vec<Client, consts::U8>;

/// The Broker Interface
///
/// This is the primary interface for devices acting as a broker.
///
/// Currently the max capacity is fixed with a maximum of 8
/// clients connected. Each Client may subscribe up to 8 topics.
/// Each Client may register up to 8 shortcodes.
///
/// In the future, these limits may be configurable.
///
/// As a note, the Broker currently creates a sizable object, due
/// to the fixed upper limits
#[derive(Default)]
pub struct Broker {
    clients: ClientStore,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ServerError {
    ClientAlreadyRegistered,
    UnknownClient,
    ClientDisconnected,
    ConnectionError,
    ResourcesExhausted,
    UnknownShortcode,
    InternalError,
}

pub const RESET_MESSAGE: Arbitrator = Arbitrator::Control(AControl {
    response: Err(ControlError::ResetConnection),
    seq: 0,
});

// Public Interfaces
impl Broker {
    /// Create a new broker with no clients attached
    #[inline(always)]
    pub fn new() -> Self {
        Broker::default()
    }

    /// Register a client to the broker
    ///
    /// This can be done dynamically, e.g. when a client connects for the
    /// first time, e.g. a TCP session is established, or the first packet
    /// is received, or can be done ahead-of-time, e.g. when communicating
    /// with a fixed set of wired devices.
    ///
    /// Clients must be registered before messages from them can be processed.
    ///
    /// If an already-registered client is re-registered, they will be reset to
    /// an initial connection state, dropping all subscriptions or shortcodes.
    pub fn register_client(&mut self, id: &Uuid) -> Result<(), ServerError> {
        if self.clients.iter().find(|c| &c.id == id).is_none() {
            self.clients
                .push(Client {
                    id: *id,
                    state: ClientState::SessionEstablished,
                })
                .map_err(|_| ServerError::ResourcesExhausted)?;
            Ok(())
        } else {
            Err(ServerError::ClientAlreadyRegistered)
        }
    }

    /// Remove a client from the broker
    ///
    /// This could be necessary if the connection to a client breaks or times out
    /// Once removed, no further messages to or from this client will be processed
    pub fn remove_client(&mut self, id: &Uuid) -> Result<(), ServerError> {
        let pos = self
            .clients
            .iter()
            .position(|c| &c.id == id)
            .ok_or(ServerError::UnknownClient)?;
        self.clients.swap_remove(pos);
        Ok(())
    }

    /// Reset a client registered with the broker, without removing it
    ///
    /// This could be necessary if the connection to a client breaks or times out.
    pub fn reset_client(&mut self, id: &Uuid) -> Result<(), ServerError> {
        let mut client = self.client_by_id_mut(id)?;
        client.state = ClientState::SessionEstablished;
        Ok(())
    }

    /// Process a single message from a client
    ///
    /// A message from a client will be processed. If processing this message
    /// generates responses that need to be sent (e.g. a publish occurs and
    /// subscribed clients should be notified, or if the broker is responding
    /// to a request from the client), they will be returned, and the messages
    /// should be sent to the appropriate clients.
    ///
    /// Requests and Responses are addressed by the Uuid registered for each client
    ///
    /// **NOTE**: If an error occurs, you probably should send a `RESET_MESSAGE` to
    /// that client to force them to reconnect. You may also want to `remove_client`
    /// or `reset_client`, depending on the situation. This will hopefully be handled
    /// automatically in the future.
    pub fn process_msg<'req, 'sio, 'me: 'req, SI: ServerIoIn, SO: ServerIoOut<'req>>(
        &'me mut self,
        sio_in: &'req mut SI,
        sio_out: &'sio mut SO,
    ) -> Result<(), ServerError> {
        let Request {
            source,
            msg,
        } = match sio_in.recv() {
            Ok(Some(req)) => req,
            Ok(None) => return Ok(()),
            Err(e) => {
                // TODO: Actual error handling
                match e {
                    ServerIoError::ToDo => {
                        // TODO: This is probably not always right
                        return Err(ServerError::ClientDisconnected);
                    }
                }
            }
        };

        match msg {
            Component::Control(ctrl) => {
                let client = self.client_by_id_mut(&source)?;

                if let Some(msg) = client.process_control(&ctrl)? {
                    sio_out
                        .push_response(msg)
                        .map_err(|_| ServerError::ResourcesExhausted)?;
                }
            }
            Component::PubSub(PubSub { ref path, ref ty }) => {
                match ty {
                    PubSubType::Pub { ref payload } => {
                        self.process_publish(sio_out, path, payload, source)?;
                    }
                    PubSubType::Sub => {
                        let client = self.client_by_id_mut(&source)?;
                        sio_out
                            .push_response(client.process_subscribe(path)?)
                            .map_err(|_| ServerError::ResourcesExhausted)?;
                    }
                    PubSubType::Unsub => {
                        let client = self.client_by_id_mut(&source)?;
                        client.process_unsub(path)?;
                        todo!()
                    }
                }
            },
        }

        Ok(())
    }
}

// Private interfaces
impl Broker {
    fn client_by_id_mut(&mut self, id: &Uuid) -> Result<&mut Client, ServerError> {
        self.clients
            .iter_mut()
            .find(|c| &c.id == id)
            .ok_or(ServerError::UnknownClient)
    }

    fn process_publish<'req, 'sio, 'me: 'req, SO: ServerIoOut<'req>>(
        &'me mut self,
        sio: &'sio mut SO,
        path: &PubSubPath<'req>,
        payload: &'req [u8],
        source: Uuid,
    ) -> Result<(), ServerError> {
        // TODO: Make sure we're not publishing to wildcards

        // First, find the sender's path
        let source_id = self
            .clients
            .iter()
            .filter_map(|c| c.state.as_connected().ok().map(|x| (c, x)))
            .find(|(c, _x)| c.id == source)
            .ok_or(ServerError::UnknownClient)?;
        let path = match path {
            // TODO: I need to make sure this is &'req, NOT &'path! That would only happen
            // if I had an Owned string here.
            PubSubPath::Long(lp) => {
                match lp {
                    ManagedString::Owned(_) => {
                        // So, we should never have an owned string here.
                        // Having one would severely mess up our lifetimes,
                        // and would generally be bad sauce.
                        //
                        // I should get rid of ManagedString, but until then,
                        // let's just cut off this lifetime path
                        return Err(ServerError::InternalError);
                    }
                    ManagedString::Borrow(lp) => *lp,
                }
            },
            PubSubPath::Short(sid) => {
                &source_id
                .1
                .shortcuts
                .iter()
                .find(|s| &s.short == sid)
                .ok_or(ServerError::UnknownShortcode)?
                .long
                .as_str()
            },
        };

        // Then, find all applicable destinations, max of 1 per destination
        'client: for (client, state) in self
            .clients
            .iter()
            .filter_map(|c| c.state.as_connected().ok().map(|x| (c, x)))
        {
            if client.id == source {
                // Don't send messages back to the sender
                continue;
            }

            for subt in state.subscriptions.iter() {
                if anachro_icd::matches(subt.as_str(), path) {
                    // Does the destination have a shortcut for this?
                    for short in state.shortcuts.iter() {
                        // NOTE: we use path, NOT subt, as it may contain wildcards
                        if path == short.long.as_str() {
                            let msg = Arbitrator::PubSub(Ok(arbitrator::PubSubResponse::SubMsg(
                                SubMsg {
                                    path: PubSubPath::Short(short.short),
                                    payload,
                                },
                            )));
                            sio
                                .push_response(Response {
                                    dest: client.id,
                                    msg,
                                })
                                .map_err(|_| ServerError::ResourcesExhausted)?;
                            continue 'client;
                        }
                    }

                    let msg = Arbitrator::PubSub(Ok(arbitrator::PubSubResponse::SubMsg(SubMsg {
                        path: PubSubPath::Long(Path::borrow_from_str(path)),
                        payload,
                    })));
                    sio
                        .push_response(Response {
                            dest: client.id,
                            msg,
                        })
                        .map_err(|_| ServerError::ResourcesExhausted)?;
                    continue 'client;
                }
            }
        }

        Ok(())
    }
}

struct Client {
    id: Uuid,
    state: ClientState,
}

impl Client {
    fn process_control(&mut self, ctrl: &Control) -> Result<Option<Response>, ServerError> {
        let response;

        let next = match &ctrl.ty {
            ControlType::RegisterComponent(ComponentInfo { name, version }) => match &self.state {
                ClientState::SessionEstablished | ClientState::Connected(_) => {
                    let resp = Arbitrator::Control(arbitrator::Control {
                        seq: ctrl.seq,
                        response: Ok(arbitrator::ControlResponse::ComponentRegistration(self.id)),
                    });

                    response = Some(Response {
                        dest: self.id,
                        msg: resp,
                    });

                    Some(ClientState::Connected(ConnectedState {
                        name: name
                            .try_to_owned()
                            .map_err(|_| ServerError::ResourcesExhausted)?,
                        version: *version,
                        subscriptions: Vec::new(),
                        shortcuts: Vec::new(),
                    }))
                }
            },
            ControlType::RegisterPubSubShortId(PubSubShort {
                long_name,
                short_id,
            }) => {
                let state = self.state.as_connected_mut()?;

                if long_name.contains('#') || long_name.contains('+') {
                    // TODO: How to handle wildcards + short names?
                    let resp = Arbitrator::Control(arbitrator::Control {
                        seq: ctrl.seq,
                        response: Err(arbitrator::ControlError::NoWildcardsInShorts),
                    });

                    response = Some(Response {
                        dest: self.id,
                        msg: resp,
                    });
                } else {
                    let shortcut_exists = state
                        .shortcuts
                        .iter()
                        .any(|sc| (sc.long.as_str() == *long_name) && (sc.short == *short_id));

                    if !shortcut_exists {
                        state
                            .shortcuts
                            .push(Shortcut {
                                long: Path::try_from_str(long_name).unwrap(),
                                short: *short_id,
                            })
                            .map_err(|_| ServerError::ResourcesExhausted)?;
                    }

                    let resp = Arbitrator::Control(arbitrator::Control {
                        seq: ctrl.seq,
                        response: Ok(arbitrator::ControlResponse::PubSubShortRegistration(
                            *short_id,
                        )),
                    });

                    response = Some(Response {
                        dest: self.id,
                        msg: resp,
                    });
                }

                // TODO: Dupe check?

                None
            }
        };

        if let Some(next) = next {
            self.state = next;
        }

        Ok(response)
    }

    fn process_subscribe<'a, 'b>(&mut self, path: &'a PubSubPath<'b>) -> Result<Response<'b>, ServerError> {
        let state = self.state.as_connected_mut()?;

        // Determine canonical path
        let path_str = match path {
            PubSubPath::Long(lp) => lp.as_str(),
            PubSubPath::Short(sid) => state
                .shortcuts
                .iter()
                .find(|s| &s.short == sid)
                .ok_or(ServerError::UnknownShortcode)?
                .long
                .as_str(),
        };

        // Only push if not a dupe
        if state
            .subscriptions
            .iter()
            .find(|s| s.as_str() == path_str)
            .is_none()
        {
            state
                .subscriptions
                .push(Path::try_from_str(path_str).unwrap())
                .map_err(|_| ServerError::ResourcesExhausted)?;
        }

        let resp = Arbitrator::PubSub(Ok(arbitrator::PubSubResponse::SubAck {
            path: path.clone(),
        }));

        Ok(Response {
            dest: self.id,
            msg: resp,
        })
    }

    fn process_unsub(&mut self, _path: &PubSubPath) -> Result<(), ServerError> {
        let _state = self.state.as_connected_mut()?;

        todo!()
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum ClientState {
    SessionEstablished,
    Connected(ConnectedState),
}

impl ClientState {
    fn as_connected(&self) -> Result<&ConnectedState, ServerError> {
        match self {
            ClientState::Connected(state) => Ok(state),
            _ => Err(ServerError::ClientDisconnected),
        }
    }

    fn as_connected_mut(&mut self) -> Result<&mut ConnectedState, ServerError> {
        match self {
            ClientState::Connected(ref mut state) => Ok(state),
            _ => Err(ServerError::ClientDisconnected),
        }
    }
}

#[derive(Debug)]
struct ConnectedState {
    name: Name<'static>,
    version: Version,
    subscriptions: Vec<Path<'static>, consts::U8>,
    shortcuts: Vec<Shortcut, consts::U8>,
}

#[derive(Debug)]
struct Shortcut {
    long: Path<'static>,
    short: u16,
}

/// A request FROM the Client, TO the Broker
///
/// This message is addressed by a UUID used when registering the client
pub struct Request<'a> {
    pub source: Uuid,
    pub msg: Component<'a>,
}

/// A response TO the Client, FROM the Broker
///
/// This message is addressed by a UUID used when registering the client
pub struct Response<'a> {
    pub dest: Uuid,
    pub msg: Arbitrator<'a>,
}

pub enum ServerIoError {
    ToDo,
}

pub trait ServerIoIn {
    fn recv<'a, 'b: 'a>(&'b mut self) -> Result<Option<Request<'b>>, ServerIoError>;
}

pub trait ServerIoOut<'resp> {
    fn push_response(&mut self, resp: Response<'resp>) -> Result<(), ServerIoError>;
}
