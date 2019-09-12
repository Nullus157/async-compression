use std::{
    io::{Error, ErrorKind, Result},
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use flate2::Compression;
use futures::{ready, stream::Stream};
use pin_project::unsafe_project;

use crate::codec::{self, Decoder, Encoder};

const OUTPUT_BUFFER_SIZE: usize = 8_000;

#[derive(Debug)]
enum State {
    WritingHeader,
    Reading,
    Writing,
    Flushing,
    WritingFooter,
    Done,
    Invalid,
}

#[derive(Debug)]
enum DeState {
    ReadingHeader,
    Reading,
    Writing,
    Flushing,
    CheckingFooter,
    Done,
    Invalid,
}

/// A gzip encoder, or compressor.
///
/// This structure implements a [`Stream`] interface and will read uncompressed data from an
/// underlying stream and emit a stream of compressed data.
#[unsafe_project(Unpin)]
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "gzip")))]
pub struct GzipEncoder<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    state: State,
    input: Bytes,
    output: BytesMut,
    encoder: codec::GzipEncoder,
}

/// A gzip decoder, or decompressor.
///
/// This structure implements a [`Stream`] interface and will read compressed data from an
/// underlying stream and emit a stream of uncompressed data.
#[unsafe_project(Unpin)]
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "gzip")))]
pub struct GzipDecoder<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    state: DeState,
    input: Bytes,
    output: BytesMut,
    decoder: codec::GzipDecoder,
}

impl<S: Stream<Item = Result<Bytes>>> GzipEncoder<S> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    pub fn new(stream: S, level: Compression) -> GzipEncoder<S> {
        GzipEncoder {
            inner: stream,
            state: State::WritingHeader,
            input: Bytes::new(),
            output: BytesMut::new(),
            encoder: codec::GzipEncoder::new(level),
        }
    }

    /// Acquires a reference to the underlying stream that this encoder is wrapping.
    pub fn get_ref(&self) -> &S {
        &self.inner
    }

    /// Acquires a mutable reference to the underlying stream that this encoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the stream which may
    /// otherwise confuse this encoder.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Acquires a pinned mutable reference to the underlying stream that this encoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the stream which may
    /// otherwise confuse this encoder.
    pub fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut S> {
        self.project().inner
    }

    /// Consumes this encoder returning the underlying stream.
    ///
    /// Note that this may discard internal state of this encoder, so care should be taken
    /// to avoid losing resources when this is called.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Stream<Item = Result<Bytes>>> GzipDecoder<S> {
    /// Creates a new decoder which will read compressed data from the given stream and emit an
    /// uncompressed stream.
    pub fn new(stream: S) -> GzipDecoder<S> {
        GzipDecoder {
            inner: stream,
            state: DeState::ReadingHeader,
            input: Bytes::new(),
            output: BytesMut::new(),
            decoder: codec::GzipDecoder::new(),
        }
    }

    /// Acquires a reference to the underlying stream that this decoder is wrapping.
    pub fn get_ref(&self) -> &S {
        &self.inner
    }

    /// Acquires a mutable reference to the underlying stream that this decoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the stream which may
    /// otherwise confuse this decoder.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Acquires a pinned mutable reference to the underlying stream that this decoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the stream which may
    /// otherwise confuse this decoder.
    pub fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut S> {
        self.project().inner
    }

    /// Consumes this decoder returning the underlying stream.
    ///
    /// Note that this may discard internal state of this decoder, so care should be taken
    /// to avoid losing resources when this is called.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Stream<Item = Result<Bytes>>> Stream for GzipEncoder<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        let mut this = self.project();

        #[allow(clippy::never_loop)] // https://github.com/rust-lang/rust-clippy/issues/4058
        loop {
            break match mem::replace(this.state, State::Invalid) {
                State::WritingHeader => {
                    this.output.resize(OUTPUT_BUFFER_SIZE, 0);
                    let output_len = this.encoder.write_header(&mut this.output)?;
                    *this.state = State::Reading;
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::Reading => {
                    *this.state = State::Reading;
                    *this.state = match ready!(this.inner.as_mut().poll_next(cx)) {
                        Some(chunk) => {
                            if this.input.is_empty() {
                                *this.input = chunk?;
                            } else {
                                this.input.extend_from_slice(&chunk?);
                            }
                            State::Writing
                        }
                        None => State::Flushing,
                    };
                    continue;
                }

                State::Writing => {
                    this.output.resize(OUTPUT_BUFFER_SIZE, 0);

                    let (done, input_len, output_len) =
                        this.encoder.encode(&this.input, &mut this.output)?;

                    *this.state = if done { State::Reading } else { State::Writing };

                    this.input.advance(input_len as usize);
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::Flushing => {
                    this.output.resize(OUTPUT_BUFFER_SIZE, 0);

                    let (done, output_len) = this.encoder.flush(&mut this.output)?;

                    *this.state = if done {
                        State::WritingFooter
                    } else {
                        State::Flushing
                    };
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::WritingFooter => {
                    this.output.resize(OUTPUT_BUFFER_SIZE, 0);

                    let output_len = this.encoder.write_footer(&mut this.output)?;
                    *this.state = State::Done;
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                State::Done => Poll::Ready(None),

                State::Invalid => panic!("GzipEncoder reached invalid state"),
            };
        }
    }
}

impl<S: Stream<Item = Result<Bytes>>> Stream for GzipDecoder<S> {
    type Item = Result<Bytes>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        let mut this = self.project();

        #[allow(clippy::never_loop)] // https://github.com/rust-lang/rust-clippy/issues/4058
        loop {
            break match mem::replace(this.state, DeState::Invalid) {
                DeState::ReadingHeader => {
                    *this.state = DeState::ReadingHeader;
                    *this.state = match ready!(this.inner.as_mut().poll_next(cx)) {
                        Some(chunk) => {
                            this.input.extend_from_slice(&chunk?);
                            if this.input.len() >= 10 {
                                let header = this.input.split_to(10);
                                if header[0..3] != [0x1f, 0x8b, 0x08] {
                                    return Poll::Ready(Some(Err(Error::new(
                                        ErrorKind::InvalidData,
                                        "Invalid file header",
                                    ))));
                                }
                                DeState::Writing
                            } else {
                                DeState::ReadingHeader
                            }
                        }
                        None => {
                            return Poll::Ready(Some(Err(Error::new(
                                ErrorKind::InvalidData,
                                "A valid header was not found",
                            ))));
                        }
                    };
                    continue;
                }

                DeState::Reading => {
                    *this.state = DeState::Reading;
                    *this.state = match ready!(this.inner.as_mut().poll_next(cx)) {
                        Some(chunk) => {
                            if this.input.is_empty() {
                                *this.input = chunk?;
                            } else {
                                this.input.extend_from_slice(&chunk?);
                            }
                            DeState::Writing
                        }
                        None => DeState::Flushing,
                    };
                    continue;
                }

                DeState::Writing => {
                    if this.output.len() < OUTPUT_BUFFER_SIZE {
                        this.output.resize(OUTPUT_BUFFER_SIZE, 0);
                    }

                    let (done, input_len, output_len) =
                        this.decoder.decode(&this.input, &mut this.output)?;

                    *this.state = if done {
                        DeState::Reading
                    } else {
                        DeState::Writing
                    };
                    this.input.advance(input_len);
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                DeState::Flushing => {
                    if this.output.len() < OUTPUT_BUFFER_SIZE {
                        this.output.resize(OUTPUT_BUFFER_SIZE, 0);
                    }

                    let (done, output_len) = this.decoder.flush(&mut this.output)?;

                    *this.state = if done {
                        DeState::CheckingFooter
                    } else {
                        DeState::Reading
                    };
                    Poll::Ready(Some(Ok(this.output.split_to(output_len).freeze())))
                }

                DeState::CheckingFooter => {
                    this.decoder.check_footer(&this.input)?;
                    *this.state = DeState::Done;
                    Poll::Ready(None)
                }

                DeState::Done => Poll::Ready(None),

                DeState::Invalid => panic!("GzipDecoder reached invalid state"),
            };
        }
    }
}

fn _assert() {
    crate::util::_assert_send::<GzipEncoder<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>>>();
    crate::util::_assert_sync::<GzipEncoder<Pin<Box<dyn Stream<Item = Result<Bytes>> + Sync>>>>();
    crate::util::_assert_send::<GzipDecoder<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>>>();
    crate::util::_assert_sync::<GzipDecoder<Pin<Box<dyn Stream<Item = Result<Bytes>> + Sync>>>>();
}
