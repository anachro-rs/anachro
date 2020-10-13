# Interface Control Document

Crates that contain message types that are used "over the wire". These crates are typically shared between many different devices.

ICD crates are typically `#[no_std]` to maintain compatibility with embedded devices, and often depend only on `serde` for the `Deserialize` and `Serialize` traits.
