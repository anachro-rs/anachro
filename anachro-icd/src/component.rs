use crate::{PubSubPath, Version};
use serde::{Deserialize, Serialize};

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
    Pub { payload: &'a [u8] },
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
    #[serde(borrow)]
    pub name: crate::Name<'a>,
    pub version: Version,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PubSubShort<'a> {
    pub long_name: &'a str,
    pub short_id: u16,
}

#[cfg(test)]
mod test {
    use super::*;
    use postcard::{from_bytes, to_stdvec};

    #[test]
    fn ser_check() {
        let name = crate::Name::try_from_str("cool-board").unwrap();
        let version = Version {
            major: 0,
            minor: 1,
            trivial: 0,
            misc: 123,
        };

        let msg = Component::Control(Control {
            seq: 0x0504,
            ty: ControlType::RegisterComponent(ComponentInfo { name, version }),
        });

        let ser_msg = to_stdvec(&msg).unwrap();
        assert_eq!(
            &ser_msg[..],
            &[
                0x00, // Component::Control
                0x04, 0x05, // seq
                0x00, // ControlType::RegisterComponent
                0x0A, b'c', b'o', b'o', b'l', b'-', b'b', b'o', b'a', b'r', b'd',
                0x00, 0x01, 0x00, 123,
            ]
        );

        let deser_msg: Component = from_bytes(&ser_msg).unwrap();

        assert_eq!(msg, deser_msg);
    }
}
