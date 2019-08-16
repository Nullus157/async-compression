use std::{
    io::{Error, ErrorKind, Result},
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{BufMut, Bytes, BytesMut};
use flate2::{Compress, Compression, Crc, Decompress, FlushCompress, FlushDecompress, Status};
use futures::{ready, stream::Stream};
use pin_project::unsafe_project;

#[derive(Debug)]
enum State {
    WritingHeader(Compression),
    Reading,
    WritingChunk(Bytes),
    FlushingData,
    WritingFooter,
    Done,
    Invalid,
}

#[derive(Debug)]
enum DeState {
    ReadingHeader,
    Reading,
    Writing,
    ReadingFooter,
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
    output: BytesMut,
    crc: Crc,
    compress: Compress,
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
    crc: Crc,
    decompress: Decompress,
}

impl<S: Stream<Item = Result<Bytes>>> GzipEncoder<S> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    pub fn new(stream: S, level: Compression) -> GzipEncoder<S> {
        GzipEncoder {
            inner: stream,
            state: State::WritingHeader(level),
            output: BytesMut::new(),
            crc: Crc::new(),
            compress: Compress::new(level, false),
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
            crc: Crc::new(),
            decompress: Decompress::new(false),
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

        fn compress(
            compress: &mut Compress,
            input: &mut Bytes,
            output: &mut BytesMut,
            crc: &mut Crc,
            flush: FlushCompress,
        ) -> Result<(Status, Bytes)> {
            const OUTPUT_BUFFER_SIZE: usize = 8_000;

            if output.len() < OUTPUT_BUFFER_SIZE {
                output.resize(OUTPUT_BUFFER_SIZE, 0);
            }

            let (prior_in, prior_out) = (compress.total_in(), compress.total_out());
            let status = compress.compress(input, output, flush)?;
            let input_len = compress.total_in() - prior_in;
            let output_len = compress.total_out() - prior_out;

            crc.update(&input[0..input_len as usize]);
            input.advance(input_len as usize);
            Ok((status, output.split_to(output_len as usize).freeze()))
        }

        #[allow(clippy::never_loop)] // https://github.com/rust-lang/rust-clippy/issues/4058
        loop {
            break match mem::replace(this.state, State::Invalid) {
                State::WritingHeader(level) => {
                    *this.state = State::Reading;
                    Poll::Ready(Some(Ok(get_header(level))))
                }

                State::Reading => {
                    *this.state = State::Reading;
                    *this.state = match ready!(this.inner.as_mut().poll_next(cx)) {
                        Some(chunk) => State::WritingChunk(chunk?),
                        None => State::FlushingData,
                    };
                    continue;
                }

                State::WritingChunk(mut input) => {
                    if input.is_empty() {
                        *this.state = State::Reading;
                        continue;
                    }

                    let (status, chunk) = compress(
                        &mut this.compress,
                        &mut input,
                        &mut this.output,
                        &mut this.crc,
                        FlushCompress::None,
                    )?;

                    *this.state = match status {
                        Status::Ok => State::WritingChunk(input),
                        Status::StreamEnd => unreachable!(),
                        Status::BufError => panic!("unexpected BufError"),
                    };

                    Poll::Ready(Some(Ok(chunk)))
                }

                State::FlushingData => {
                    let (status, chunk) = compress(
                        &mut this.compress,
                        &mut Bytes::new(),
                        &mut this.output,
                        &mut this.crc,
                        FlushCompress::Finish,
                    )?;

                    *this.state = match status {
                        Status::StreamEnd => State::WritingFooter,
                        Status::Ok => State::FlushingData,
                        Status::BufError => panic!("unexpected BufError"),
                    };

                    Poll::Ready(Some(Ok(chunk)))
                }

                State::WritingFooter => {
                    let mut footer = BytesMut::with_capacity(8);

                    footer.put(this.crc.sum().to_le_bytes().as_ref());
                    footer.put(this.crc.amount().to_le_bytes().as_ref());

                    *this.state = State::Done;

                    Poll::Ready(Some(Ok(footer.freeze())))
                }

                State::Done => Poll::Ready(None),

                State::Invalid => panic!("GzipEncoder reached invalid state"),
            };
        }
    }
}

fn get_header(level: Compression) -> Bytes {
    let mut header = vec![0u8; 10];
    header[0] = 0x1f;
    header[1] = 0x8b;
    header[2] = 0x08;
    header[8] = if level.level() >= Compression::best().level() {
        0x02
    } else if level.level() <= Compression::fast().level() {
        0x04
    } else {
        0x00
    };
    header[9] = 0xff;

    Bytes::from(header)
}

impl<S: Stream<Item = Result<Bytes>>> Stream for GzipDecoder<S> {
    type Item = Result<Bytes>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        let mut this = self.project();

        fn decompress(
            decompress: &mut Decompress,
            input: &mut Bytes,
            output: &mut BytesMut,
            crc: &mut Crc,
            flush: FlushDecompress,
        ) -> Result<(Status, Bytes)> {
            const OUTPUT_BUFFER_SIZE: usize = 8_000;

            if output.len() < OUTPUT_BUFFER_SIZE {
                output.resize(OUTPUT_BUFFER_SIZE, 0);
            }

            let (prior_in, prior_out) = (decompress.total_in(), decompress.total_out());
            let status = decompress.decompress(input, output, flush)?;
            let input_len = decompress.total_in() - prior_in;
            let output_len = decompress.total_out() - prior_out;

            crc.update(&output[0..output_len as usize]);
            input.advance(input_len as usize);
            Ok((status, output.split_to(output_len as usize).freeze()))
        }

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
                        None => DeState::ReadingFooter,
                    };
                    continue;
                }

                DeState::Writing => {
                    if this.input.len() <= 8 {
                        *this.state = DeState::Reading;
                        continue;
                    }
                    let (status, chunk) = decompress(
                        &mut this.decompress,
                        this.input,
                        &mut this.output,
                        &mut this.crc,
                        FlushDecompress::None,
                    )?;

                    *this.state = match status {
                        Status::Ok => DeState::Writing,
                        Status::StreamEnd => DeState::Reading,
                        Status::BufError => panic!("unexpected BufError"),
                    };

                    Poll::Ready(Some(Ok(chunk)))
                }

                DeState::ReadingFooter => {
                    *this.state = DeState::Done;
                    if this.input.len() == 8 {
                        let crc = &this.crc.sum().to_le_bytes()[..];
                        let bytes_read = &this.crc.amount().to_le_bytes()[..];
                        if crc != &this.input[0..4] {
                            Poll::Ready(Some(Err(Error::new(
                                ErrorKind::InvalidData,
                                "CRC computed does not match",
                            ))))
                        } else if bytes_read != &this.input[4..8] {
                            Poll::Ready(Some(Err(Error::new(
                                ErrorKind::InvalidData,
                                "amount of bytes read does not match",
                            ))))
                        } else {
                            Poll::Ready(None)
                        }
                    } else {
                        Poll::Ready(Some(Err(Error::new(
                            ErrorKind::UnexpectedEof,
                            "reached unexpected EOF",
                        ))))
                    }
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
