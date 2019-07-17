use std::{
    fmt,
    io::Result,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use futures::{ready, stream::Stream};
use libzstd::stream::raw::{Decoder, Encoder, Operation};
use pin_project::unsafe_project;

#[derive(Debug)]
enum State {
    Reading,
    Writing(Bytes),
    Flushing,
    Done,
    Invalid,
}

#[derive(Debug)]
enum DeState {
    Reading,
    Writing(Bytes),
    Done,
    Invalid,
}

/// A zstd encoder, or compressor.
///
/// This structure implements a [`Stream`] interface and will read uncompressed data from an
/// underlying stream and emit a stream of compressed data.
#[unsafe_project(Unpin)]
#[cfg_attr(docsrs, doc(cfg(feature = "zstd")))]
pub struct ZstdEncoder<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    state: State,
    output: BytesMut,
    encoder: Encoder,
}

/// A zstd decoder, or decompressor.
///
/// This structure implements a [`Stream`] interface and will read compressed data from an
/// underlying stream and emit a stream of uncompressed data.
#[unsafe_project(Unpin)]
#[cfg_attr(docsrs, doc(cfg(feature = "zstd")))]
pub struct ZstdDecoder<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    state: DeState,
    output: BytesMut,
    decoder: Decoder,
}

impl<S: Stream<Item = Result<Bytes>>> ZstdEncoder<S> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    ///
    /// The `level` argument here can range from 1-21. A level of `0` will use zstd's default, which is `3`.
    pub fn new(stream: S, level: i32) -> ZstdEncoder<S> {
        ZstdEncoder {
            inner: stream,
            state: State::Reading,
            output: BytesMut::new(),
            encoder: Encoder::new(level).unwrap(),
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

impl<S: Stream<Item = Result<Bytes>>> ZstdDecoder<S> {
    /// Creates a new decoder which will read compressed data from the given stream and emit an
    /// uncompressed stream.
    pub fn new(stream: S) -> ZstdDecoder<S> {
        ZstdDecoder {
            inner: stream,
            state: DeState::Reading,
            output: BytesMut::new(),
            decoder: Decoder::new().unwrap(),
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

impl<S: Stream<Item = Result<Bytes>>> Stream for ZstdEncoder<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        let mut this = self.project();

        fn compress(
            encoder: &mut Encoder,
            input: &mut Bytes,
            output: &mut BytesMut,
        ) -> Result<Bytes> {
            const OUTPUT_BUFFER_SIZE: usize = 8_000;

            if output.len() < OUTPUT_BUFFER_SIZE {
                output.resize(OUTPUT_BUFFER_SIZE, 0);
            }

            let status = encoder.run_on_buffers(input, output)?;
            input.advance(status.bytes_read);
            Ok(output.split_to(status.bytes_written).freeze())
        }

        #[allow(clippy::never_loop)] // https://github.com/rust-lang/rust-clippy/issues/4058
        loop {
            break match mem::replace(this.state, State::Invalid) {
                State::Reading => {
                    *this.state = State::Reading;
                    *this.state = match ready!(this.inner.as_mut().poll_next(cx)) {
                        Some(chunk) => State::Writing(chunk?),
                        None => State::Flushing,
                    };
                    continue;
                }
                State::Writing(mut input) => {
                    if input.is_empty() {
                        *this.state = State::Reading;
                        continue;
                    }

                    let chunk = compress(&mut this.encoder, &mut input, &mut this.output)?;

                    *this.state = State::Writing(input);

                    Poll::Ready(Some(Ok(chunk)))
                }
                State::Flushing => {
                    let mut output = zstd_safe::OutBuffer::around(this.output);

                    let bytes_left = this.encoder.flush(&mut output).unwrap();
                    *this.state = if bytes_left == 0 {
                        let _ = this.encoder.finish(&mut output, true);
                        State::Done
                    } else {
                        State::Flushing
                    };
                    Poll::Ready(Some(Ok(output.as_slice().into())))
                }
                State::Done => Poll::Ready(None),
                State::Invalid => panic!("ZstdEncoder reached invalid state"),
            };
        }
    }
}

impl<S: Stream<Item = Result<Bytes>>> Stream for ZstdDecoder<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        let mut this = self.project();

        fn decompress(
            decoder: &mut Decoder,
            input: &mut Bytes,
            output: &mut BytesMut,
        ) -> Result<Bytes> {
            const OUTPUT_BUFFER_SIZE: usize = 8_000;

            if output.len() < OUTPUT_BUFFER_SIZE {
                output.resize(OUTPUT_BUFFER_SIZE, 0);
            }

            let status = decoder.run_on_buffers(input, output)?;
            input.advance(status.bytes_read);
            Ok(output.split_to(status.bytes_written).freeze())
        }

        #[allow(clippy::never_loop)] // https://github.com/rust-lang/rust-clippy/issues/4058
        loop {
            break match mem::replace(this.state, DeState::Invalid) {
                DeState::Reading => {
                    *this.state = DeState::Reading;
                    *this.state = match ready!(this.inner.as_mut().poll_next(cx)) {
                        Some(chunk) => DeState::Writing(chunk?),
                        None => DeState::Done,
                    };
                    continue;
                }
                DeState::Writing(mut input) => {
                    if input.is_empty() {
                        *this.state = DeState::Reading;
                        continue;
                    }

                    let chunk = decompress(&mut this.decoder, &mut input, &mut this.output)?;

                    *this.state = DeState::Writing(input);

                    Poll::Ready(Some(Ok(chunk)))
                }
                DeState::Done => Poll::Ready(None),
                DeState::Invalid => panic!("ZstdDecoder reached invalid state"),
            };
        }
    }
}

impl<S: Stream<Item = Result<Bytes>> + fmt::Debug> fmt::Debug for ZstdEncoder<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ZstdEncoder")
            .field("inner", &self.inner)
            .field("state", &self.state)
            .field("output", &self.output)
            .field("encoder", &"<no debug>")
            .finish()
    }
}

impl<S: Stream<Item = Result<Bytes>> + fmt::Debug> fmt::Debug for ZstdDecoder<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ZstdDecoder")
            .field("inner", &self.inner)
            .field("state", &self.state)
            .field("output", &self.output)
            .field("decoder", &"<no debug>")
            .finish()
    }
}
