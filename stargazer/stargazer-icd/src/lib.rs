#![no_std]

use serde::{Deserialize, Serialize};
use anachro_client::pubsub_table;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Keypress {
    pub character: char,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ColorMe {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pubsub_table!(
    KeyboardTable,
    // ===================
    Subs => {
        Color: "ident/led/keyboard" => ColorMe,
    },
    Pubs => {
        Key: "keyboard/keypress/printable" => Keypress,
    },
);

pubsub_table!(
    CpuTable,
    // ================
    Subs => {
        Key: "keyboard/keypress/printable" => Keypress,
    },
    Pubs => {
        IdentKeyboard: "ident/led/keyboard" => ColorMe,
        IdentDisplay: "ident/led/display" => ColorMe,
    },
);

pubsub_table!(
    DisplayTable,
    // ================
    Subs => {
        Key: "keyboard/keypress/printable" => Keypress,
    },
    Pubs => {
        IdentKeyboard: "ident/led/keyboard" => ColorMe,
        IdentDisplay: "ident/led/display" => ColorMe,
    },
);
