use crate::codecs::Decode;
use crate::core::util::PartialBuffer;

use core::task::{Context, Poll};
use std::io::Result;

use futures_core::ready;

#[derive(Debug)]
enum State {
    Decoding,
    Flushing,
    Done,
    Next,
}

pub(crate) trait AsyncBufRead {
    fn poll_fill_buf(&mut self, cx: &mut Context<'_>) -> Poll<Result<&[u8]>>;
    fn consume(&mut self, bytes: usize);
}

#[derive(Debug)]
pub struct Decoder {
    state: State,
    multiple_members: bool,
}

impl Default for Decoder {
    fn default() -> Self {
        Self {
            state: State::Decoding,
            multiple_members: false,
        }
    }
}

impl Decoder {
    pub fn multiple_members(&mut self, enabled: bool) {
        self.multiple_members = enabled;
    }

    pub fn do_poll_read<D: Decode>(
        &mut self,
        cx: &mut Context<'_>,
        output: &mut PartialBuffer<&mut [u8]>,
        reader: &mut dyn AsyncBufRead,
        decoder: &mut D,
    ) -> Poll<Result<()>> {
        let mut first = true;

        loop {
            self.state = match self.state {
                State::Decoding => {
                    let input = if first {
                        &[][..]
                    } else {
                        ready!(reader.poll_fill_buf(cx))?
                    };

                    if input.is_empty() && !first {
                        // Avoid attempting to reinitialise the decoder if the
                        // reader has returned EOF.
                        self.multiple_members = false;

                        State::Flushing
                    } else {
                        let mut input = PartialBuffer::new(input);
                        let res = decoder.decode(&mut input, output).or_else(|err| {
                            // ignore the first error, occurs when input is empty
                            // but we need to run decode to flush
                            if first {
                                Ok(false)
                            } else {
                                Err(err)
                            }
                        });

                        if !first {
                            let len = input.written().len();
                            reader.consume(len);
                        }

                        first = false;

                        if res? {
                            State::Flushing
                        } else {
                            State::Decoding
                        }
                    }
                }

                State::Flushing => {
                    if decoder.finish(output)? {
                        if self.multiple_members {
                            decoder.reinit()?;
                            State::Next
                        } else {
                            State::Done
                        }
                    } else {
                        State::Flushing
                    }
                }

                State::Done => State::Done,

                State::Next => {
                    let input = ready!(reader.poll_fill_buf(cx))?;
                    if input.is_empty() {
                        State::Done
                    } else {
                        State::Decoding
                    }
                }
            };

            if let State::Done = self.state {
                return Poll::Ready(Ok(()));
            }
            if output.unwritten().is_empty() {
                return Poll::Ready(Ok(()));
            }
        }
    }
}
