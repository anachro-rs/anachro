use core::mem::MaybeUninit;
pub use generic_array::typenum::consts;
pub use generic_array::{ArrayLength, GenericArray};
use postcard;
use serde::de::{Deserialize, DeserializeOwned};

#[derive(Default)]
pub struct Buffer<N: ArrayLength<u8>> {
    pub(crate) buf: GenericArray<u8, N>,
    idx: usize,
}

pub enum FeedResult<'a, T> {
    /// Consumed all data, still pending
    Consumed,

    /// Buffer was filled. Contains remaining section of input, if any
    OverFull(&'a [u8]),

    /// Reached end of chunk, but deserialization failed. Contains
    /// remaining section of input, if any
    DeserError(&'a [u8]),

    /// Deserialization complete. Contains deserialized data and
    /// remaining section of input, if any
    Success { data: T, remaining: &'a [u8] },
}

pub enum SimpleResult<'input, 'buffer> {
    /// Consumed all data, still pending
    Consumed,

    /// Buffer was filled. Contains remaining section of input, if any
    OverFull(&'input [u8]),

    /// Deframing complete. Contains deserialized data and
    /// remaining section of input, if any
    Success {
        data: &'buffer mut [u8],
        remaining: &'input [u8],
    },
}

pub enum WithResult<'a, R> {
    /// Consumed all data, still pending
    Consumed,

    /// Buffer was filled. Contains remaining section of input, if any
    OverFull(&'a [u8]),

    /// Reached end of chunk, but deserialization failed. Contains
    /// remaining section of input, if any
    DeserError(&'a [u8]),

    /// Deserialization complete. Contains deserialized data and
    /// remaining section of input, if any
    SuccessWith { result: R, remaining: &'a [u8] },
}

impl<N> Buffer<N>
where
    N: ArrayLength<u8>,
{
    pub fn new() -> Self {
        Buffer {
            buf: unsafe { MaybeUninit::zeroed().assume_init() },
            idx: 0,
        }
    }

    pub fn feed_with<'a: 'me, 'de, 'me: 'de, T, F, R>(
        &'me mut self,
        input: &'a [u8],
        fun: F,
    ) -> WithResult<'a, R>
    where
        T: 'de + Deserialize<'de>,
        F: FnOnce(T) -> R,
        R: 'static,
    {
        if input.is_empty() {
            return WithResult::Consumed;
        }

        let zero_pos = input.iter().position(|&i| i == 0);

        if let Some(n) = zero_pos {
            // Yes! We have an end of message here.
            // Add one to include the zero in the "take" portion
            // of the buffer, rather than in "release".
            let (take, release) = input.split_at(n + 1);

            // Does it fit?
            if (self.idx + n) <= N::to_usize() {
                // Aw yiss - add to array
                self.extend_unchecked(take);

                let retval = match postcard::from_bytes_cobs::<T>(&mut self.buf[..self.idx]) {
                    Ok(t) => WithResult::SuccessWith {
                        remaining: release,
                        result: fun(t),
                    },
                    Err(_) => WithResult::DeserError(release),
                };
                self.idx = 0;
                retval
            } else {
                self.idx = 0;
                WithResult::OverFull(release)
            }
        } else {
            // Does it fit?
            if (self.idx + input.len()) > N::to_usize() {
                // nope
                let new_start = N::to_usize() - self.idx;
                self.idx = 0;
                WithResult::OverFull(&input[new_start..])
            } else {
                // yup!
                self.extend_unchecked(input);
                WithResult::Consumed
            }
        }
    }

    pub fn feed<'a, T: DeserializeOwned>(&mut self, input: &'a [u8]) -> FeedResult<'a, T> {
        if input.is_empty() {
            return FeedResult::Consumed;
        }

        let zero_pos = input.iter().position(|&i| i == 0);

        if let Some(n) = zero_pos {
            // Yes! We have an end of message here.
            // Add one to include the zero in the "take" portion
            // of the buffer, rather than in "release".
            let (take, release) = input.split_at(n + 1);

            // Does it fit?
            if (self.idx + n) <= N::to_usize() {
                // Aw yiss - add to array
                self.extend_unchecked(take);

                let retval = match postcard::from_bytes_cobs::<T>(&mut self.buf[..self.idx]) {
                    Ok(t) => FeedResult::Success {
                        data: t,
                        remaining: release,
                    },
                    Err(_) => FeedResult::DeserError(release),
                };
                self.idx = 0;
                retval
            } else {
                self.idx = 0;
                FeedResult::OverFull(release)
            }
        } else {
            // Does it fit?
            if (self.idx + input.len()) > N::to_usize() {
                // nope
                let new_start = N::to_usize() - self.idx;
                self.idx = 0;
                FeedResult::OverFull(&input[new_start..])
            } else {
                // yup!
                self.extend_unchecked(input);
                FeedResult::Consumed
            }
        }
    }

    pub fn feed_simple<'input, 'buffer>(
        &'buffer mut self,
        input: &'input [u8],
    ) -> SimpleResult<'input, 'buffer> {
        if input.is_empty() {
            return SimpleResult::Consumed;
        }

        let zero_pos = input.iter().position(|&i| i == 0);

        if let Some(n) = zero_pos {
            // Yes! We have an end of message here.
            // Add one to include the zero in the "take" portion
            // of the buffer, rather than in "release".
            let (take, release) = input.split_at(n + 1);

            // Does it fit?
            if (self.idx + n) <= N::to_usize() {
                // Aw yiss - add to array
                self.extend_unchecked(take);

                let retval = SimpleResult::Success {
                    data: &mut self.buf[..self.idx],
                    remaining: release,
                };

                self.idx = 0;
                retval
            } else {
                self.idx = 0;
                SimpleResult::OverFull(release)
            }
        } else {
            // Does it fit?
            if (self.idx + input.len()) > N::to_usize() {
                // nope
                let new_start = N::to_usize() - self.idx;
                self.idx = 0;
                SimpleResult::OverFull(&input[new_start..])
            } else {
                // yup!
                self.extend_unchecked(input);
                SimpleResult::Consumed
            }
        }
    }

    /// extend the internal buffer with the given input. Will panic
    /// if the input does not fit in the internal buffer.
    fn extend_unchecked(&mut self, input: &[u8]) {
        let new_end = self.idx + input.len();
        self.buf.as_mut_slice()[self.idx..new_end].copy_from_slice(input);
        self.idx = new_end;
    }
}

#[test]
fn loop_test() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    struct Demo {
        a: u32,
        b: u8,
    }

    let mut raw_buf = [0u8; 64];
    let mut cobs_buf: Buffer<consts::U64> = Buffer::new();

    let ser = postcard::to_slice_cobs(&Demo { a: 10, b: 20 }, &mut raw_buf).unwrap();

    if let FeedResult::Success { data, remaining } = cobs_buf.feed(ser) {
        assert_eq!(Demo { a: 10, b: 20 }, data);
        assert_eq!(remaining.len(), 0);
    } else {
        panic!()
    }
}
