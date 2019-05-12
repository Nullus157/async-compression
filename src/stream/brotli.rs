use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::{Error, ErrorKind, Result};

use brotli2::{
    raw::{CoStatus, Compress, CompressOp, DeStatus, Decompress},
    CompressParams,
};
use bytes::{Bytes, BytesMut};
use futures::{ready, stream::Stream};
use pin_project::unsafe_project;

/// A brotli encoder, or compressor.
///
/// This structure implements a [`Stream`] interface and will read uncompressed data from an
/// underlying stream and emit a stream of compressed data.
#[unsafe_project(Unpin)]
pub struct BrotliEncoder<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    flush: bool,
    compress: Compress,
}

/// A brotli decoder, or decompressor.
///
/// This structure implements a [`Stream`] interface and will read compressed data from an
/// underlying stream and emit a stream of uncompressed data.
#[unsafe_project(Unpin)]
pub struct BrotliDecoder<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    flush: bool,
    decompress: Decompress,
}

impl<S: Stream<Item = Result<Bytes>>> BrotliEncoder<S> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    ///
    /// The `level` argument here is typically 0-11.
    pub fn new(stream: S, level: u32) -> BrotliEncoder<S> {
        let mut params = CompressParams::new();
        params.quality(level);
        BrotliEncoder::from_params(stream, &params)
    }

    /// Creates a new encoder with a custom [`CompressParams`].
    pub fn from_params(stream: S, params: &CompressParams) -> BrotliEncoder<S> {
        let mut compress = Compress::new();
        compress.set_params(params);
        BrotliEncoder {
            inner: stream,
            flush: false,
            compress,
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

impl<S: Stream<Item = Result<Bytes>>> BrotliDecoder<S> {
    /// Creates a new decoder which will read compressed data from the given stream and emit an
    /// uncompressed stream.
    pub fn new(stream: S) -> BrotliDecoder<S> {
        BrotliDecoder {
            inner: stream,
            flush: false,
            decompress: Decompress::new(),
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

impl<S: Stream<Item = Result<Bytes>>> Stream for BrotliEncoder<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        const OUTPUT_BUFFER_SIZE: usize = 8_000;

        let this = self.project();

        if *this.flush {
            return Poll::Ready(None);
        }

        let input_buffer = if let Some(bytes) = ready!(this.inner.poll_next(cx)) {
            bytes?
        } else {
            *this.flush = true;
            Bytes::new()
        };

        let mut compressed_output = BytesMut::with_capacity(OUTPUT_BUFFER_SIZE);
        let input_ref = &mut input_buffer.as_ref();
        let output_ref = &mut &mut [][..];
        loop {
            let status = this.compress.compress(
                if *this.flush {
                    CompressOp::Finish
                } else {
                    CompressOp::Process
                },
                input_ref,
                output_ref,
            )?;
            while let Some(buf) = this.compress.take_output(None) {
                compressed_output.extend_from_slice(buf);
            }
            match status {
                CoStatus::Finished => break,
                CoStatus::Unfinished => (),
            }
        }

        Poll::Ready(Some(Ok(compressed_output.freeze())))
    }
}

impl<S: Stream<Item = Result<Bytes>>> Stream for BrotliDecoder<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        const OUTPUT_BUFFER_SIZE: usize = 8_000;

        let this = self.project();

        if *this.flush {
            return Poll::Ready(None);
        }

        let input_buffer = if let Some(bytes) = ready!(this.inner.poll_next(cx)) {
            bytes?
        } else {
            *this.flush = true;
            Bytes::new()
        };

        let mut decompressed_output = BytesMut::with_capacity(OUTPUT_BUFFER_SIZE);
        let input_ref = &mut input_buffer.as_ref();
        let output_ref = &mut &mut [][..];
        loop {
            let status = this.decompress.decompress(input_ref, output_ref)?;
            while let Some(buf) = this.decompress.take_output(None) {
                decompressed_output.extend_from_slice(buf);
            }
            match status {
                DeStatus::Finished => break,
                DeStatus::NeedInput => {
                    if *this.flush {
                        return Poll::Ready(Some(Err(Error::new(
                            ErrorKind::UnexpectedEof,
                            "reached unexpected EOF",
                        ))));
                    }
                    break;
                }
                DeStatus::NeedOutput => (),
            }
        }

        Poll::Ready(Some(Ok(decompressed_output.freeze())))
    }
}
