#![no_std]

pub mod arbitrator;
pub mod component;

use embedded_dma::{ReadBuffer, WriteBuffer};

#[derive(Debug)]
pub(crate) struct ConstRawSlice {
    ptr: *const u8,
    len: usize,
}

#[derive(Debug)]
pub(crate) struct MutRawSlice {
    ptr: *mut u8,
    len: usize,
}

unsafe impl WriteBuffer for MutRawSlice {
    type Word = u8;

    unsafe fn write_buffer(&mut self) -> (*mut Self::Word, usize) {
        (self.ptr, self.len)
    }
}

unsafe impl ReadBuffer for ConstRawSlice {
    type Word = u8;

    unsafe fn read_buffer(&self) -> (*const Self::Word, usize) {
        (self.ptr, self.len)
    }
}
