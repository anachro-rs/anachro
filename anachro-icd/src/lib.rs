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
