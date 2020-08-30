//! # Anachro ICD
//!
//! This is the Interface Control Document (ICD) for the Anachro PC
//! communication protocol.
//!
//! This library defines the types used on by clients and servers of
//! the anachro protocol.
//!
//! This library is currently primarily focused on a Pub/Sub style
//! protocol, but will add support for Object Store and Mailbox
//! style communication in the future.
#![no_std]

use core::{hash::Hash, marker::PhantomData, str::FromStr};
use heapless::{consts, ArrayLength, String};
use serde::{
    de::{Deserializer, Visitor},
    ser::Serializer,
    Deserialize, Serialize,
};

pub mod arbitrator;
pub mod component;

/// A type alias for the Maximum Pub/Sub Path
pub type MaxPathLen = consts::U127;

/// A type alias for the maximum device name
pub type MaxNameLen = consts::U32;

/// Publish/Subscribe Path - Short or Long
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub enum PubSubPath<'a> {
    /// A long form, UTF-8 Path
    #[serde(borrow)]
    Long(Path<'a>),

    /// A short form, 'memoized' path
    ///
    /// Pre-registered with the server
    Short(u16),
}

/// Device version - SemVer
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone, Copy)]
pub struct Version {
    /// Major Semver Version
    pub major: u8,

    /// Minor Semver Version
    pub minor: u8,

    /// Trivial Semver Version
    pub trivial: u8,

    /// Misc Version
    ///
    /// Could be build number, etc.
    pub misc: u8,
}

/// A Pub/Sub Path as a Managed String
pub type Path<'a> = ManagedString<'a, MaxPathLen>;

/// A device name as a Managed String
pub type Name<'a> = ManagedString<'a, MaxNameLen>;

/// A borrowed or owned string
///
/// Basically like CoW, but with heapless::String
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ManagedString<'a, T>
where
    T: ArrayLength<u8>,
{
    Owned(String<T>),
    Borrow(&'a str),
}

impl<'a, T> Serialize for ManagedString<'a, T>
where
    T: ArrayLength<u8>,
{
    /// We can serialize our managed string as a str
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'a, 'de: 'a, T> Deserialize<'de> for ManagedString<'a, T>
where
    T: ArrayLength<u8> + Sized,
{
    /// We can deserialize as a borrowed str
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ManagedStringVisitor::new())
    }
}

struct ManagedStringVisitor<'a, T>
where
    T: ArrayLength<u8> + Sized,
{
    _t: PhantomData<T>,
    _lt: PhantomData<&'a ()>,
}

impl<'a, T> ManagedStringVisitor<'a, T>
where
    T: ArrayLength<u8> + Sized,
{
    fn new() -> ManagedStringVisitor<'a, T> {
        Self {
            _t: PhantomData,
            _lt: PhantomData,
        }
    }
}

impl<'de: 'a, 'a, T> Visitor<'de> for ManagedStringVisitor<'a, T>
where
    T: ArrayLength<u8> + Sized,
{
    type Value = ManagedString<'a, T>;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(formatter, "a borrowable str")
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // Always borrow
        Ok(ManagedString::Borrow(value))
    }
}

impl<'a, T> ManagedString<'a, T>
where
    T: ArrayLength<u8>,
{
    /// Obtain self as a string slice
    pub fn as_str(&self) -> &str {
        match self {
            ManagedString::Owned(o) => o.as_str(),
            ManagedString::Borrow(s) => s,
        }
    }

    pub fn as_borrowed(&'a self) -> ManagedString<'a, T> {
        ManagedString::Borrow(self.as_str())
    }

    /// Attempt to create an Owned string from a given str
    ///
    /// May fail if the heapless::String is not large enough to
    /// contain this slice
    pub fn try_from_str(input: &str) -> Result<ManagedString<'static, T>, ()> {
        Ok(ManagedString::Owned(String::from_str(input)?))
    }

    /// Create a Borrowed string from a given str
    pub fn borrow_from_str<'i>(input: &'i str) -> ManagedString<'i, T> {
        ManagedString::Borrow(input)
    }

    /// Attempt to convert to an Owned string from the current state
    ///
    /// May fail if the heapless::String is not large enough to
    /// contain this slice
    pub fn try_to_owned(&self) -> Result<ManagedString<'static, T>, ()> {
        ManagedString::try_from_str(self.as_str())
    }
}

/// A UUID as a block of 16 bytes
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone, Copy, Hash)]
pub struct Uuid([u8; 16]);

impl Uuid {
    /// Create a new UUID from an array of bytes
    pub fn from_bytes(by: [u8; 16]) -> Self {
        Uuid(by)
    }

    /// Obtain the UUID contents as a borrowed array of bytes
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Obtain the UUID contents as a slice of bytes
    pub fn as_slice(&self) -> &[u8] {
        &self.0[..]
    }
}
