use uuid::Uuid;
use serde::{Serialize, Deserialize};
use crate::PubSubPath;

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
    SubMsg {
        #[serde(borrow)]
        path: PubSubPath<'a>,
        payload: &'a [u8],
    },
    GetMsg {
        #[serde(borrow)]
        path: PubSubPath<'a>,
        payload: &'a [u8],
    },
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Control {
    seq: u16,
    response: Result<ControlResponse, ControlError>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ControlResponse {
    ComponentRegistration(Uuid),
    PubSubShortRegistration(u16),
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ControlError {

}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum PubSubError {

}


#[cfg(test)]
mod test {
    use super::*;
    use postcard::{to_stdvec, from_bytes};

    #[test]
    fn ser_check() {
        let uuid = Uuid::parse_str("d036e73b-23ec-4f60-accb-0eddb617f471").unwrap();
        let msg = Arbitrator::Control(Control {
            seq: 0x0405,
            response: Ok(ControlResponse::ComponentRegistration(
                uuid
            ))
        });

        let ser_msg = to_stdvec(&msg).unwrap();
        assert_eq!(
            &ser_msg[..],
            &[
                0x00,       // Arbitrator::Control
                0x05, 0x04, // seq
                0x00,       // OK
                0x00,       // ControlResponse::ComponentRegistration
                0x10,       // Len: 16
                0xd0, 0x36, 0xe7, 0x3b,
                0x23, 0xec,
                0x4f, 0x60,
                0xac, 0xcb,
                0x0e, 0xdd, 0xb6, 0x17, 0xf4, 0x71,
            ],
        );

        let deser_msg: Arbitrator = from_bytes(&ser_msg).unwrap();
        assert_eq!(deser_msg, msg);
    }
}
