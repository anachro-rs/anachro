pub mod arbitrator;
pub mod component;
use serde::{Deserialize, Serialize};

// TODO: Switch to "managed" crate for everything here?

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub enum PubSubPath<'a> {
    #[serde(borrow)]
    Long(&'a str),
    Short(u16),
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone, Copy)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub trivial: u8,
    pub misc: u8,
}
