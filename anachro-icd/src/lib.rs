#![no_std]

pub mod arbitrator;
pub mod component;
use core::str::FromStr;
use serde::{Deserialize, ser::Serializer, de::{Deserializer, Visitor}, Serialize};
use heapless::{String, consts, ArrayLength};

// TODO: Switch to "managed" crate for everything here?

pub type MaxPathLen = consts::U127;
pub type MaxNameLen = consts::U32;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub enum PubSubPath<'a> {
    #[serde(borrow)]
    Long(Path<'a>),
    Short(u16),
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone, Copy)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub trivial: u8,
    pub misc: u8,
}

pub type Path<'a> = ManagedString<'a, MaxPathLen>;
pub type Name<'a> = ManagedString<'a, MaxNameLen>;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ManagedString<'a, T>
where
    T: ArrayLength<u8>
{
    Owned(String<T>),
    Borrow(&'a str),
}

impl<'a, T> Serialize for ManagedString<'a, T>
where
    T: ArrayLength<u8>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'a, 'de: 'a, T> Deserialize<'de> for ManagedString<'a, T>
where
    T: ArrayLength<u8> + Sized
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        // ManagedString::Borrow(deserializer.deserialize_str(serde::de:))
        deserializer.deserialize_str(ManagedStringVisitor::new())
    }
}

use core::marker::PhantomData;

struct ManagedStringVisitor<'a, T>
where
    T: ArrayLength<u8> + Sized
{
    _t: PhantomData<T>,
    _lt: PhantomData<&'a ()>,
}

impl<'a, T> ManagedStringVisitor<'a, T>
where
    T: ArrayLength<u8> + Sized
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
    T: ArrayLength<u8> + Sized
{
    type Value = ManagedString<'a, T>;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        todo!()
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error
    {
        Ok(ManagedString::Borrow(value))
    }
}

impl<'a, T> ManagedString<'a, T>
where
    T: ArrayLength<u8>
{
    fn as_str(&self) -> &str {
        match self {
            ManagedString::Owned(o) => o.as_str(),
            ManagedString::Borrow(s) => s
        }
    }

    pub fn try_from_str(input: &str) -> Result<ManagedString<'static, T>, ()> {
        Ok(ManagedString::Owned(String::from_str(input)?))
    }

    pub fn borrow_from_str<'i>(input: &'i str) -> ManagedString<'i, T> {
        ManagedString::Borrow(input)
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone, Copy)]
pub struct Uuid([u8; 16]);

impl Uuid {
    pub fn from_bytes(by: [u8; 16]) -> Self {
        Uuid(by)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0[..]
    }
}
