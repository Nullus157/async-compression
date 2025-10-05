use crate::codecs::Decode;
use crate::core::util::PartialBuffer;

use std::{io::Result, ops::ControlFlow};

#[derive(Debug)]
enum State {
    Decoding,
    Flushing,
    Done,
    Next,
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
        output: &mut PartialBuffer<&mut [u8]>,
        decoder: &mut D,
        input: &mut PartialBuffer<&[u8]>,
        mut first: bool,
    ) -> ControlFlow<Result<()>> {
        loop {
            self.state = match self.state {
                State::Decoding => {
                    if input.unwritten().is_empty() && !first {
                        // Avoid attempting to reinitialise the decoder if the
                        // reader has returned EOF.
                        self.multiple_members = false;

                        State::Flushing
                    } else {
                        match decoder.decode(input, output) {
                            Ok(true) => State::Flushing,
                            // ignore the first error, occurs when input is empty
                            // but we need to run decode to flush
                            Err(err) if !first => return ControlFlow::Break(Err(err)),
                            // poll for more data for the next decode
                            _ => break,
                        }
                    }
                }

                State::Flushing => {
                    match decoder.finish(output) {
                        Ok(true) => {
                            if self.multiple_members {
                                if let Err(err) = decoder.reinit() {
                                    return ControlFlow::Break(Err(err));
                                }

                                // The decode stage might consume all the input,
                                // the next stage might need to poll again if it's empty.
                                first = true;
                                State::Next
                            } else {
                                State::Done
                            }
                        }
                        Ok(false) => State::Flushing,
                        Err(err) => return ControlFlow::Break(Err(err)),
                    }
                }

                State::Done => return ControlFlow::Break(Ok(())),

                State::Next => {
                    if input.unwritten().is_empty() {
                        if first {
                            // poll for more data to check if there's another stream
                            break;
                        }
                        State::Done
                    } else {
                        State::Decoding
                    }
                }
            };

            if output.unwritten().is_empty() {
                return ControlFlow::Break(Ok(()));
            }
        }

        if output.unwritten().is_empty() {
            ControlFlow::Break(Ok(()))
        } else {
            ControlFlow::Continue(())
        }
    }
}

macro_rules! impl_do_poll_read {
    () => {
        use crate::generic::bufread::Decoder as GenericDecoder;

        use std::ops::ControlFlow;

        use futures_core::ready;
        use pin_project_lite::pin_project;

        pin_project! {
            #[derive(Debug)]
            pub struct Decoder<R, D> {
                #[pin]
                reader: R,
                decoder: D,
                inner: GenericDecoder,
            }
        }

        impl<R: AsyncBufRead, D: Decode> Decoder<R, D> {
            pub fn new(reader: R, decoder: D) -> Self {
                Self {
                    reader,
                    decoder,
                    inner: GenericDecoder::default(),
                }
            }
        }

        impl<R, D> Decoder<R, D> {
            pub fn get_ref(&self) -> &R {
                &self.reader
            }

            pub fn get_mut(&mut self) -> &mut R {
                &mut self.reader
            }

            pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut R> {
                self.project().reader
            }

            pub fn into_inner(self) -> R {
                self.reader
            }

            pub fn multiple_members(&mut self, enabled: bool) {
                self.inner.multiple_members(enabled);
            }
        }

        impl<R: AsyncBufRead, D: Decode> Decoder<R, D> {
            fn do_poll_read(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                output: &mut PartialBuffer<&mut [u8]>,
            ) -> Poll<Result<()>> {
                let mut this = self.project();

                if let ControlFlow::Break(res) = this.inner.do_poll_read(
                    output,
                    this.decoder,
                    &mut PartialBuffer::new(&[][..]),
                    true,
                ) {
                    return Poll::Ready(res);
                }

                loop {
                    let mut input =
                        PartialBuffer::new(ready!(this.reader.as_mut().poll_fill_buf(cx))?);

                    let control_flow =
                        this.inner
                            .do_poll_read(output, this.decoder, &mut input, false);

                    let bytes_read = input.written().len();
                    this.reader.as_mut().consume(bytes_read);

                    if let ControlFlow::Break(res) = control_flow {
                        break Poll::Ready(res);
                    }
                }
            }
        }
    };
}
pub(crate) use impl_do_poll_read;
