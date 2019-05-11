use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;
use std::marker::Unpin;

use super::flate::{FlateDecoder, FlateEncoder};
use bytes::Bytes;
use flate2::{Compress, Compression, Decompress};
use futures::{stream::Stream, stream::StreamExt};

pub struct ZlibEncoder<S: Stream<Item = Result<Bytes>> + Unpin> {
    inner: FlateEncoder<S>,
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> Stream for ZlibEncoder<S> {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        self.inner.poll_next_unpin(cx)
    }
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> ZlibEncoder<S> {
    pub fn new(stream: S, level: Compression) -> ZlibEncoder<S> {
        ZlibEncoder {
            inner: FlateEncoder::new(stream, Compress::new(level, true)),
        }
    }
}

pub struct ZlibDecoder<S: Stream<Item = Result<Bytes>> + Unpin> {
    inner: FlateDecoder<S>,
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> Stream for ZlibDecoder<S> {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        self.inner.poll_next_unpin(cx)
    }
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> ZlibDecoder<S> {
    pub fn new(stream: S) -> ZlibDecoder<S> {
        ZlibDecoder {
            inner: FlateDecoder::new(stream, Decompress::new(true)),
        }
    }
}
