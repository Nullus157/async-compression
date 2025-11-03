use crate::{codecs::Encode, core::util::PartialBuffer};
use std::{io::Result, ops::ControlFlow};

#[derive(Debug)]
enum State {
    Encoding(usize),
    Flushing,
    Finishing,
    Done,
}

#[derive(Debug)]
pub struct Encoder {
    state: State,
}

impl Default for Encoder {
    fn default() -> Self {
        Self {
            state: State::Encoding(0),
        }
    }
}

impl Encoder {
    /// `input` - should be `None` if `Poll::Pending`.
    pub fn do_poll_read(
        &mut self,
        output: &mut PartialBuffer<&mut [u8]>,
        encoder: &mut impl Encode,
        mut input: Option<&mut PartialBuffer<&[u8]>>,
    ) -> ControlFlow<Result<()>> {
        loop {
            self.state = match self.state {
                State::Encoding(mut read) => match input.as_mut() {
                    None => {
                        if read == 0 {
                            // Poll for more data
                            // TODO (nobodyxu): Return Ok if `!output.written().is_empty()`
                            break;
                        } else {
                            State::Flushing
                        }
                    }
                    Some(input) => {
                        if input.unwritten().is_empty() {
                            State::Finishing
                        } else {
                            if let Err(err) = encoder.encode(input, output) {
                                return ControlFlow::Break(Err(err));
                            }

                            read += input.written().len();

                            // Poll for more data
                            break;
                        }
                    }
                },

                State::Flushing => match encoder.flush(output) {
                    Ok(true) => {
                        self.state = State::Encoding(0);

                        // Poll for more data
                        break;
                    }
                    Ok(false) => State::Flushing,
                    Err(err) => return ControlFlow::Break(Err(err)),
                },

                State::Finishing => match encoder.finish(output) {
                    Ok(true) => State::Done,
                    Ok(false) => State::Finishing,
                    Err(err) => return ControlFlow::Break(Err(err)),
                },

                State::Done => return ControlFlow::Break(Ok(())),
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

macro_rules! impl_encoder {
    () => {
        use crate::generic::bufread::Encoder as GenericEncoder;

        use std::ops::ControlFlow;

        use futures_core::ready;
        use pin_project_lite::pin_project;

        pin_project! {
            #[derive(Debug)]
            pub struct Encoder<R, E> {
                #[pin]
                reader: R,
                encoder: E,
                inner: GenericEncoder,
            }
        }

        impl<R: AsyncBufRead, E: Encode> Encoder<R, E> {
            pub fn new(reader: R, encoder: E) -> Self {
                Self {
                    reader,
                    encoder,
                    inner: Default::default(),
                }
            }

            pub fn with_capacity(reader: R, encoder: E, _cap: usize) -> Self {
                Self::new(reader, encoder)
            }
        }

        impl<R, E> Encoder<R, E> {
            pub fn get_ref(&self) -> &R {
                &self.reader
            }

            pub fn get_mut(&mut self) -> &mut R {
                &mut self.reader
            }

            pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut R> {
                self.project().reader
            }

            pub(crate) fn get_encoder_ref(&self) -> &E {
                &self.encoder
            }

            pub fn into_inner(self) -> R {
                self.reader
            }
        }

        impl<R: AsyncBufRead, E: Encode> Encoder<R, E> {
            fn do_poll_read(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                output: &mut PartialBuffer<&mut [u8]>,
            ) -> Poll<Result<()>> {
                let mut this = self.project();

                // TODO (nobodyxu): Maybe do_poll_read with input as `None`, to flush any
                // existing data ASAP instead of waiting for poll_fill_buf

                loop {
                    let mut input = match this.reader.as_mut().poll_fill_buf(cx) {
                        Poll::Pending => None,
                        Poll::Ready(res) => Some(PartialBuffer::new(res?)),
                    };

                    let control_flow =
                        this.inner
                            .do_poll_read(output, &mut *this.encoder, input.as_mut());

                    let is_pending = input.is_none();
                    if let Some(input) = input {
                        let len = input.written().len();
                        this.reader.as_mut().consume(len);
                    }

                    if let ControlFlow::Break(res) = control_flow {
                        break Poll::Ready(res);
                    }

                    if is_pending {
                        return Poll::Pending;
                    }
                }
            }
        }
    };
}
pub(crate) use impl_encoder;
