use serde::{Serialize, Deserialize};
use crate::PubSubPath;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Component<'a> {
    #[serde(borrow)]
    Control(Control<'a>),

    #[serde(borrow)]
    PubSub(PubSub<'a>),
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PubSub<'a> {
    #[serde(borrow)]
    pub path: PubSubPath<'a>,
    pub ty: PubSubType<'a>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum PubSubType<'a> {
    Pub {
        payload: &'a [u8],
    },
    // TODO: Periodic option for sub? min/max rate?
    Sub,
    Unsub,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Control<'a> {
    pub seq: u16,
    #[serde(borrow)]
    pub ty: ControlType<'a>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ControlType<'a> {
    #[serde(borrow)]
    RegisterComponent(ComponentInfo<'a>),
    #[serde(borrow)]
    RegisterPubSubShortId(PubSubShort<'a>),
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ComponentInfo<'a> {
    pub name: &'a str,
    pub version: &'a str,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PubSubShort<'a> {
    pub long_name: &'a str,
    pub short_id: u16,
}

#[cfg(test)]
mod test {
    use super::*;
    use postcard::{to_stdvec, from_bytes};

    #[test]
    fn ser_check() {
        let name = "cool-board";
        let version = "v0.1.0";

        let msg = Component::Control(Control {
            seq: 0x0504,
            ty: ControlType::RegisterComponent(ComponentInfo {
                name,
                version,
            })
        });

        let ser_msg = to_stdvec(&msg).unwrap();
        assert_eq!(
            &ser_msg[..],
            &[
                0x00,       // Component::Control
                0x04, 0x05, // seq
                0x00,       // ControlType::RegisterComponent
                0x0A, b'c', b'o', b'o', b'l', b'-', b'b', b'o', b'a', b'r', b'd',
                0x06, b'v', b'0', b'.', b'1', b'.', b'0',
            ]
        );

        let deser_msg: Component = from_bytes(&ser_msg).unwrap();

        assert_eq!(msg, deser_msg);
    }
}
