pub mod component;
pub mod arbitrator;
use serde::{Serialize, Deserialize};

// TODO: Switch to "managed" crate for everything here?

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub enum PubSubPath<'a> {
    #[serde(borrow)]
    Long(&'a str),
    Short(u16),
}
