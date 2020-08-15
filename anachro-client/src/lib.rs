/*

Thoughts:

* How to handle active pass/fail requests, like a pending subscription?
* Separate types for pre-registration?

*/

use anachro_icd::{
    arbitrator::{
        Arbitrator, Control as AControl, ControlResponse, PubSubError, PubSubResponse, SubMsg,
    },
    component::{Component, ComponentInfo, Control as CControl, ControlType, PubSub, PubSubType, PubSubShort},
    PubSubPath,
};
use uuid::Uuid;

pub struct Client {
    state: ClientState,
    name: String,
    version: String,
    ctr: u16,
}

pub enum ClientState {
    Created,
    PendingRegistration,
    Active(ActiveState),
}

impl ClientState {
    fn as_active_mut(&mut self) -> Result<&mut ActiveState, ()> {
        match self {
            ClientState::Active(a_state) => Ok(a_state),
            _ => Err(()),
        }
    }

    fn as_active(&self) -> Result<&ActiveState, ()> {
        match self {
            ClientState::Active(a_state) => Ok(a_state),
            _ => Err(()),
        }
    }
}

pub struct ActiveState {
    uuid: Uuid,
    pending_sub: bool,
    pending_short: Option<u16>,
}

pub struct Deliverables<'a> {
    pub broker_response: Option<Component<'a>>,
    pub client_response: Option<SubMsg<'a>>,
}

impl Client {
    pub fn new(name: String, version: String, ctr_init: u16) -> Self {
        Self {
            name,
            version,
            ctr: ctr_init,
            state: ClientState::Created
        }
    }

    pub fn is_connected(&self) -> bool {
        self.state.as_active().is_ok()
    }

    pub fn subscribe<'a, 'b: 'a>(
        &mut self,
        path: PubSubPath<'a>,
    ) -> Result<Component<'a>, ()> {
        // Only possible if we are already connected
        let state = self.state.as_active_mut()?;

        // TODO, we could track multiple pending subs in the
        // future, at the cost of storing a vec of pending subs
        if state.pending_sub {
            return Err(());
        }

        state.pending_sub = true;
        Ok(Component::PubSub(PubSub {
            path,
            ty: PubSubType::Sub
        }))
    }

    pub fn is_subscribe_pending(&self) -> bool {
        if let Ok(state) = self.state.as_active() {
            state.pending_sub
        } else {
            false
        }
    }

    pub fn is_reg_short_pending(&self) -> bool {
        if let Ok(state) = self.state.as_active() {
            state.pending_short.is_some()
        } else {
            false
        }
    }

    pub fn publish<'a, 'b: 'a>(&'b self, path: PubSubPath<'a>, payload: &'a [u8]) -> Result<Component<'a>, ()> {
        self.state.as_active()?;

        Ok(Component::PubSub(PubSub {
            path,
            ty: PubSubType::Pub {
                payload
            }
        }))
    }

    pub fn register_short<'a, 'b: 'a>(
        &'b mut self,
        short: u16,
        long: &'a str
    ) -> Result<Component<'a>, ()> {
        // Only possible if we are already connected
        let state = self.state.as_active_mut()?;

        if state.pending_short.is_some() {
            return Err(());
        }

        state.pending_short = Some(short);

        self.ctr += 1;

        Ok(Component::Control(
            CControl {
                seq: self.ctr,
                ty: ControlType::RegisterPubSubShortId(PubSubShort {
                    long_name: long,
                    short_id: short,
                })
            }
        ))
    }

    pub fn process<'a, 'b: 'a>(
        &'b mut self,
        msg: &'a Option<Arbitrator<'a>>,
    ) -> Result<Deliverables<'a>, ()> {
        let mut response = Deliverables {
            broker_response: None,
            client_response: None,
        };

        let next = match (&mut self.state, msg) {
            (ClientState::Created, _) => {
                self.ctr += 1;

                response.broker_response = Some(Component::Control(CControl {
                    seq: self.ctr,
                    ty: ControlType::RegisterComponent(ComponentInfo {
                        name: &self.name,
                        version: &self.version,
                    }),
                }));

                Some(ClientState::PendingRegistration)
            }
            (ClientState::PendingRegistration, Some(msg)) => {
                if let Arbitrator::Control(AControl { seq, response }) = msg {
                    if *seq != self.ctr {
                        // TODO, restart connection process? Just disregard?
                        return Err(());
                    }
                    if let Ok(ControlResponse::ComponentRegistration(uuid)) = response {
                        Some(ClientState::Active(ActiveState {
                            uuid: *uuid,
                            pending_sub: false,
                            pending_short: None,
                        }))
                    } else {
                        // TODO, restart connection process? Just disregard?
                        return Err(());
                    }
                } else {
                    None
                }
            }
            (ClientState::PendingRegistration, None) => {
                // TODO: Some kind of timeout? Just wait forever?
                None
            }
            (ClientState::Active(ref mut a_state), Some(Arbitrator::Control(ref ctl)))
                if ctl.seq == self.ctr =>
            {
                // TODO: Can this generate any kind of response to the user or broker? Update state?
                a_state.process_control(ctl)?;
                None
            }
            (ClientState::Active(ref mut a_state), Some(Arbitrator::PubSub(ref ps))) => {
                response.client_response = a_state.process_pubsub(ps)?;
                None
            }
            (ClientState::Active(_), Some(_)) => {
                // TODO: Process any other kind of message?
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
    fn process_control(&mut self, msg: &AControl) -> Result<(), ()> {
        match msg.response {
            Ok(ControlResponse::ComponentRegistration(_)) => {
                // We already registered?
                return Err(());
            }
            Ok(ControlResponse::PubSubShortRegistration(short_id)) => {
                if let Some(exp_id) = self.pending_short {
                    if exp_id != short_id {
                        // This wasn't the shortcode response we were expecting
                        return Err(());
                    }
                } else {
                    // We weren't expecting a shortcode response?
                    return Err(());
                }
                // We got what we were expecting! Clear it
                self.pending_short = None;
            }
            Err(_) => {
                // ?
                return Err(());
            }
        }

        Ok(())
    }

    fn process_pubsub<'a>(
        &mut self,
        msg: &'a Result<PubSubResponse, PubSubError>,
    ) -> Result<Option<SubMsg<'a>>, ()> {
        match msg {
            Ok(PubSubResponse::SubAck { .. }) => {
                if self.pending_sub {
                    // TODO: Check we're getting the right subscription?
                    self.pending_sub = false;
                    Ok(None)
                } else {
                    Err(())
                }
            }
            Ok(PubSubResponse::SubMsg(SubMsg { path, payload })) => Ok(Some(SubMsg {
                path: path.clone(),
                payload,
            })),
            Err(_) => Err(()),
        }
    }
}
