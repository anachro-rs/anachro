/*

Thoughts:

* How to handle active pass/fail requests, like a pending subscription?
* Separate types for pre-registration?

*/

use uuid::Uuid;
use anachro_icd::{
    arbitrator::{Arbitrator, Control as AControl, ControlResponse},
    component::{Component, Control as CControl, ControlType, ComponentInfo},
    PubSubPath,
};

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

pub struct ActiveState {
    uuid: Uuid,
    pending_sub: bool,
    pending_short: Option<u16>,
}

pub struct Deliverables<'a> {
    broker_response: Option<Component<'a>>,
    api_response: Option<()>,
}

impl Client {
    pub fn process(&mut self, msg: Option<Arbitrator>) -> Result<Deliverables, ()> {
        let mut response = Deliverables {
            broker_response: None,
            api_response: None,
        };
        let next = match (&self.state, msg) {
            (ClientState::Created, _) => {
                self.ctr += 1;

                response.broker_response = Some(Component::Control(CControl {
                    seq: self.ctr,
                    ty: ControlType::RegisterComponent(ComponentInfo {
                        name: &self.name,
                        version: &self.version,
                    })
                }));

                Some(ClientState::PendingRegistration)
            }
            (ClientState::PendingRegistration, Some(msg)) => {
                if let Arbitrator::Control(AControl {
                    seq,
                    response,
                }) = msg {
                    if seq != self.ctr {
                        // TODO, restart connection process? Just disregard?
                        return Err(());
                    }
                    if let Ok(ControlResponse::ComponentRegistration(uuid)) = response {
                        Some(ClientState::Active(ActiveState {
                            uuid,
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
            (ClientState::Active(a_state), Some(ref msg)) => {
                match msg {
                    Arbitrator::Control(ctl) => {
                        if !a_state.pending_sub && a_state.pending_short.is_none() {
                            // We didn't ask for no stinking control messages
                            return Err(());
                        }
                        todo!()
                    },
                    Arbitrator::PubSub(ps) => todo!(),
                    _ => todo!(),
                }
            }
            (ClientState::Active(a_state), None) => {
                // Todo: any periodic keepalive pings?
                None
            }
        };

        if let Some(state) = next {
            self.state = state;
        }

        Ok(response)
    }
}
