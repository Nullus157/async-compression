use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;

use super::flate::{FlateDecoder, FlateEncoder};
use bytes::Bytes;
use flate2::{Compress, Compression, Decompress};
use futures::stream::Stream;
use pin_project::unsafe_project;

/// A zlib encoder, or compressor.
///
/// This structure implements a [`Stream`] interface and will read uncompressed data from an
/// underlying stream and emit a stream of compressed data.
#[unsafe_project(Unpin)]
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "zlib")))]
pub struct ZlibEncoder<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: FlateEncoder<S>,
}

/// A zlib decoder, or decompressor.
///
/// This structure implements a [`Stream`] interface and will read compressed data from an
/// underlying stream and emit a stream of uncompressed data.
#[unsafe_project(Unpin)]
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "zlib")))]
pub struct ZlibDecoder<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: FlateDecoder<S>,
}

impl<S: Stream<Item = Result<Bytes>>> ZlibEncoder<S> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    pub fn new(stream: S, level: Compression) -> ZlibEncoder<S> {
        ZlibEncoder {
            inner: FlateEncoder::new(stream, Compress::new(level, true)),
        }
    }

    /// Acquires a reference to the underlying stream that this encoder is wrapping.
    pub fn get_ref(&self) -> &S {
        self.inner.get_ref()
    }

    /// Acquires a mutable reference to the underlying stream that this encoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the stream which may
    /// otherwise confuse this encoder.
    pub fn get_mut(&mut self) -> &mut S {
        self.inner.get_mut()
    }

    /// Acquires a pinned mutable reference to the underlying stream that this encoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the stream which may
    /// otherwise confuse this encoder.
    pub fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut S> {
        self.project().inner.get_pin_mut()
    }

    /// Consumes this encoder returning the underlying stream.
    ///
    /// Note that this may discard internal state of this encoder, so care should be taken
    /// to avoid losing resources when this is called.
    pub fn into_inner(self) -> S {
        self.inner.into_inner()
    }
}

impl<S: Stream<Item = Result<Bytes>>> ZlibDecoder<S> {
    /// Creates a new decoder which will read compressed data from the given stream and emit an
    /// uncompressed stream.
    pub fn new(stream: S) -> ZlibDecoder<S> {
        ZlibDecoder {
            inner: FlateDecoder::new(stream, Decompress::new(true)),
        }
    }

    /// Acquires a reference to the underlying stream that this decoder is wrapping.
    pub fn get_ref(&self) -> &S {
        self.inner.get_ref()
    }

    /// Acquires a mutable reference to the underlying stream that this decoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the stream which may
    /// otherwise confuse this decoder.
    pub fn get_mut(&mut self) -> &mut S {
        self.inner.get_mut()
    }

    /// Acquires a pinned mutable reference to the underlying stream that this decoder is wrapping.
    ///
    /// Note that care must be taken to avoid tampering with the state of the stream which may
    /// otherwise confuse this decoder.
    pub fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut S> {
        self.project().inner.get_pin_mut()
    }

    /// Consumes this decoder returning the underlying stream.
    ///
    /// Note that this may discard internal state of this decoder, so care should be taken
    /// to avoid losing resources when this is called.
    pub fn into_inner(self) -> S {
        self.inner.into_inner()
    }
}

impl<S: Stream<Item = Result<Bytes>>> Stream for ZlibEncoder<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        self.project().inner.poll_next(cx)
    }
}

impl<S: Stream<Item = Result<Bytes>>> Stream for ZlibDecoder<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        self.project().inner.poll_next(cx)
    }
}

fn _assert() {
    crate::util::_assert_send::<ZlibEncoder<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>>>();
    crate::util::_assert_sync::<ZlibEncoder<Pin<Box<dyn Stream<Item = Result<Bytes>> + Sync>>>>();
    crate::util::_assert_send::<ZlibDecoder<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>>>();
    crate::util::_assert_sync::<ZlibDecoder<Pin<Box<dyn Stream<Item = Result<Bytes>> + Sync>>>>();
}
