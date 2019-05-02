use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::Result;
use std::marker::Unpin;

use super::flate::{CompressedStream, DecompressedStream};
use bytes::Bytes;
use flate2::{Compress, Decompress};
pub use flate2::Compression;
use futures::{stream::Stream, stream::StreamExt};

pub struct DeflateStream<S: Stream<Item = Result<Bytes>> + Unpin> {
    inner: CompressedStream<S>,
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> Stream for DeflateStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        self.inner.poll_next_unpin(cx)
    }
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> DeflateStream<S> {
    pub fn new(stream: S, level: Compression) -> DeflateStream<S> {
        DeflateStream {
            inner: CompressedStream::new(stream, Compress::new(level, false)),
        }
    }
}

pub struct DecompressedDeflateStream<S: Stream<Item = Result<Bytes>> + Unpin> {
    inner: DecompressedStream<S>,
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> Stream for DecompressedDeflateStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        self.inner.poll_next_unpin(cx)
    }
}

impl<S: Stream<Item = Result<Bytes>> + Unpin> DecompressedDeflateStream<S> {
    pub fn new(stream: S) -> DecompressedDeflateStream<S> {
        DecompressedDeflateStream {
            inner: DecompressedStream::new(stream, Decompress::new(false)),
        }
    }
}
