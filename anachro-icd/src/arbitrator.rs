use crate::PubSubPath;
use serde::{Deserialize, Serialize};
use crate::Uuid;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Arbitrator<'a> {
    Control(Control),
    #[serde(borrow)]
    PubSub(Result<PubSubResponse<'a>, PubSubError>),
    ObjStore,
    Mailbox,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum PubSubResponse<'a> {
    SubAck {
        #[serde(borrow)]
        path: PubSubPath<'a>,
    },
    SubMsg(SubMsg<'a>),
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SubMsg<'a> {
    #[serde(borrow)]
    pub path: PubSubPath<'a>,
    pub payload: &'a [u8],
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Control {
    pub seq: u16,
    pub response: Result<ControlResponse, ControlError>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ControlResponse {
    ComponentRegistration(Uuid),
    PubSubShortRegistration(u16),
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ControlError {}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum PubSubError {}

#[cfg(test)]
mod test {
    use super::*;
    use postcard::{from_bytes, to_stdvec};

    #[test]
    fn ser_check() {
        let uuid = Uuid::from_bytes([0xd0, 0x36, 0xe7, 0x3b, 0x23, 0xec, 0x4f, 0x60, 0xac, 0xcb, 0x0e, 0xdd, 0xb6, 0x17, 0xf4, 0x71]);
        let msg = Arbitrator::Control(Control {
            seq: 0x0405,
            response: Ok(ControlResponse::ComponentRegistration(uuid)),
        });

        let ser_msg = to_stdvec(&msg).unwrap();
        assert_eq!(
            &ser_msg[..],
            &[
                0x00, // Arbitrator::Control
                0x05, 0x04, // seq
                0x00, // OK
                0x00, // ControlResponse::ComponentRegistration
                0x10, // Len: 16
                0xd0, 0x36, 0xe7, 0x3b, 0x23, 0xec, 0x4f, 0x60, 0xac, 0xcb, 0x0e, 0xdd, 0xb6, 0x17,
                0xf4, 0x71,
            ],
        );

        let deser_msg: Arbitrator = from_bytes(&ser_msg).unwrap();
        assert_eq!(deser_msg, msg);
    }
}
