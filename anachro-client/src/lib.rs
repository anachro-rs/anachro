/*

Thoughts:

* How to handle active pass/fail requests, like a pending subscription?
* Separate types for pre-registration?

*/

use uuid::Uuid;
use anachro_icd::{
    arbitrator::{Arbitrator, Control as AControl, ControlResponse, PubSubResponse, PubSubError, SubMsg},
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

struct Message {
    path: String,
    payload: Vec<u8>,
}

pub struct Deliverables<'a> {
    broker_response: Option<Component<'a>>,
    client_response: Option<SubMsg<'a>>,
}

impl Client {
    pub fn process<'a, 'b: 'a>(&'b mut self, msg: &'a Option<Arbitrator<'a>>) -> Result<Deliverables<'a>, ()> {
        let mut response = Deliverables {
            broker_response: None,
            client_response: None,
        };


        let next = {
            let Client {
                ref mut state,
                ref name,
                ref version,
                ctr,
            } = self;

            match (state, msg) {
            (ClientState::Created, _) => {
                *ctr += 1;

                response.broker_response = Some(Component::Control(CControl {
                    seq: self.ctr,
                    ty: ControlType::RegisterComponent(ComponentInfo {
                        name,
                        version,
                    })
                }));

                Some(ClientState::PendingRegistration)
            }
            (ClientState::PendingRegistration, Some(msg)) => {
                if let Arbitrator::Control(AControl {
                    seq,
                    response,
                }) = msg {
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
            (ClientState::Active(ref mut a_state), Some(Arbitrator::Control(ref ctl))) if ctl.seq == self.ctr => {
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
        }};

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

    fn process_pubsub<'a>(&mut self, msg: &'a Result<PubSubResponse, PubSubError>) -> Result<Option<SubMsg<'a>>, ()> {
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
            Ok(PubSubResponse::SubMsg(SubMsg { path, payload })) => {
                Ok(Some(SubMsg { path: path.clone(), payload }))
            }
            Err(_) => {
                Err(())
            }
        }
    }
}
