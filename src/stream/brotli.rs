use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io::{Error, ErrorKind, Result};

use brotli2::raw::{CoStatus, CompressOp, DeStatus, Decompress};
pub use brotli2::{raw::Compress, CompressParams};
use bytes::{Bytes, BytesMut};
use futures::{ready, stream::Stream};
use pin_project::unsafe_project;

#[unsafe_project(Unpin)]
pub struct BrotliStream<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    flush: bool,
    compress: Compress,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for BrotliStream<S> {
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

impl<S: Stream<Item = Result<Bytes>>> BrotliStream<S> {
    pub fn new(stream: S, compress: Compress) -> BrotliStream<S> {
        BrotliStream {
            inner: stream,
            flush: false,
            compress,
        }
    }
}

#[unsafe_project(Unpin)]
pub struct DecompressedBrotliStream<S: Stream<Item = Result<Bytes>>> {
    #[pin]
    inner: S,
    flushing: bool,
    decompress: Decompress,
}

impl<S: Stream<Item = Result<Bytes>>> Stream for DecompressedBrotliStream<S> {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
//        const OUTPUT_BUFFER_SIZE: usize = 8_000;
//
//        let this = self.project();
//
//        if *this.flushing {
//            return Poll::Ready(None);
//        }
//
//        let input_buffer = if let Some(bytes) = ready!(this.inner.poll_next(cx)) {
//            bytes?
//        } else {
//            *this.flushing = true;
//            Bytes::new()
//        };
//
//        let mut decompressed_output = BytesMut::with_capacity(OUTPUT_BUFFER_SIZE);
//        let input_ref = &mut input_buffer.as_ref();
//        let output_ref = &mut &mut [][..];
//        loop {
//            let status = this.decompress.decompress(input_ref, output_ref)?;
//            while let Some(buf) = this.decompress.take_output(None) {
//                decompressed_output.put(buf);
//            }
//            match status {
//                DeStatus::Finished => break,
//                DeStatus::NeedInput => {
//                    if *this.flushing {
//                        return Poll::Ready(Some(Err(Error::new(
//                            ErrorKind::UnexpectedEof,
//                            "reached unexpected EOF",
//                        ))));
//                    }
//                    break;
//                }
//                DeStatus::NeedOutput => (),
//            }
//        }
//
//        Poll::Ready(Some(Ok(decompressed_output.freeze())))
        unimplemented!()
    }
}

impl<S: Stream<Item = Result<Bytes>>> DecompressedBrotliStream<S> {
    pub fn new(stream: S) -> DecompressedBrotliStream<S> {
        DecompressedBrotliStream {
            inner: stream,
            flushing: false,
            decompress: Decompress::new(),
        }
    }
}
