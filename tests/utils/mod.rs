#![allow(unused)] // Different tests use a different subset of functions

use bytes::Bytes;
use futures::{
    executor::{block_on, block_on_stream},
    io::{AsyncBufRead, AsyncRead, AsyncReadExt},
    stream::{self, Stream, TryStreamExt},
};
use futures_test::{io::AsyncReadTestExt, stream::StreamTestExt};
use pin_project::unsafe_project;
use pin_utils::pin_mut;
use proptest_derive::Arbitrary;
use std::{
    io::{self, Cursor, Read},
    pin::Pin,
    task::{Context, Poll},
    vec,
};

#[derive(Arbitrary, Debug)]
pub struct InputStream(Vec<Vec<u8>>);

impl InputStream {
    pub fn stream(&self) -> impl Stream<Item = io::Result<Bytes>> {
        // The resulting stream here will interleave empty chunks before and after each chunk, and
        // then interleave a `Poll::Pending` between each yielded chunk, that way we test the
        // handling of these two conditions in every point of the tested stream.
        stream::iter(
            self.0
                .clone()
                .into_iter()
                .map(Bytes::from)
                .flat_map(|bytes| vec![Bytes::new(), bytes])
                .chain(Some(Bytes::new()))
                .map(Ok),
        )
        .interleave_pending()
    }

    pub fn reader(&self) -> impl AsyncBufRead {
        // TODO: By using the stream here we ensure that each chunk will require a separate
        // read/poll_fill_buf call to process to help test reading multiple chunks. This is
        // blocked on fixing AsyncBufRead for IntoAsyncRead:
        // (https://github.com/rust-lang-nursery/futures-rs/pull/1595)
        Cursor::new(self.bytes()).interleave_pending()
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.0.iter().flatten().cloned().collect()
    }
}

// This happens to be the only dimension we're using
impl From<[[u8; 3]; 2]> for InputStream {
    fn from(input: [[u8; 3]; 2]) -> InputStream {
        InputStream(vec![Vec::from(&input[0][..]), Vec::from(&input[1][..])])
    }
}
impl From<Vec<Vec<u8>>> for InputStream {
    fn from(input: Vec<Vec<u8>>) -> InputStream {
        InputStream(input)
    }
}

fn read_to_vec(mut read: impl Read) -> Vec<u8> {
    let mut output = vec![];
    read.read_to_end(&mut output).unwrap();
    output
}

fn async_read_to_vec(mut read: impl AsyncRead) -> Vec<u8> {
    let mut output = vec![];
    pin_mut!(read);
    block_on(read.read_to_end(&mut output)).unwrap();
    output
}

fn stream_to_vec(stream: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
    pin_mut!(stream);
    block_on_stream(stream)
        .map(Result::unwrap)
        .flatten()
        .collect()
}

pub fn brotli_compress(bytes: &[u8]) -> Vec<u8> {
    use brotli2::bufread::BrotliEncoder;
    read_to_vec(BrotliEncoder::new(bytes, 1))
}

pub fn brotli_decompress(bytes: &[u8]) -> Vec<u8> {
    use brotli2::bufread::BrotliDecoder;
    read_to_vec(BrotliDecoder::new(bytes))
}

pub fn brotli_stream_compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
    use async_compression::stream::BrotliEncoder;
    pin_mut!(input);
    stream_to_vec(BrotliEncoder::new(input, 1))
}

pub fn brotli_stream_decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
    use async_compression::stream::BrotliDecoder;
    pin_mut!(input);
    stream_to_vec(BrotliDecoder::new(input))
}

pub fn deflate_compress(bytes: &[u8]) -> Vec<u8> {
    use flate2::{bufread::DeflateEncoder, Compression};
    read_to_vec(DeflateEncoder::new(bytes, Compression::fast()))
}

pub fn deflate_decompress(bytes: &[u8]) -> Vec<u8> {
    use flate2::bufread::DeflateDecoder;
    read_to_vec(DeflateDecoder::new(bytes))
}

pub fn deflate_stream_compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
    use async_compression::{flate2::Compression, stream::DeflateEncoder};
    pin_mut!(input);
    stream_to_vec(DeflateEncoder::new(input, Compression::fast()))
}

pub fn deflate_stream_decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
    use async_compression::stream::DeflateDecoder;
    pin_mut!(input);
    stream_to_vec(DeflateDecoder::new(input))
}

pub fn deflate_bufread_compress(input: impl AsyncBufRead) -> Vec<u8> {
    use async_compression::{bufread::DeflateEncoder, flate2::Compression};
    pin_mut!(input);
    async_read_to_vec(DeflateEncoder::new(input, Compression::fast()))
}

pub fn zlib_compress(bytes: &[u8]) -> Vec<u8> {
    use flate2::{bufread::ZlibEncoder, Compression};
    read_to_vec(ZlibEncoder::new(bytes, Compression::fast()))
}

pub fn zlib_decompress(bytes: &[u8]) -> Vec<u8> {
    use flate2::bufread::ZlibDecoder;
    read_to_vec(ZlibDecoder::new(bytes))
}

pub fn zlib_stream_compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
    use async_compression::{flate2::Compression, stream::ZlibEncoder};
    pin_mut!(input);
    stream_to_vec(ZlibEncoder::new(input, Compression::fast()))
}

pub fn zlib_stream_decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
    use async_compression::stream::ZlibDecoder;
    pin_mut!(input);
    stream_to_vec(ZlibDecoder::new(input))
}

pub fn zlib_bufread_compress(input: impl AsyncBufRead) -> Vec<u8> {
    use async_compression::{bufread::ZlibEncoder, flate2::Compression};
    pin_mut!(input);
    async_read_to_vec(ZlibEncoder::new(input, Compression::fast()))
}

pub fn gzip_compress(bytes: &[u8]) -> Vec<u8> {
    use flate2::{bufread::GzEncoder, Compression};
    read_to_vec(GzEncoder::new(bytes, Compression::fast()))
}

pub fn gzip_decompress(bytes: &[u8]) -> Vec<u8> {
    use flate2::bufread::GzDecoder;
    read_to_vec(GzDecoder::new(bytes))
}

pub fn gzip_stream_compress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
    use async_compression::{flate2::Compression, stream::GzipEncoder};
    pin_mut!(input);
    stream_to_vec(GzipEncoder::new(input, Compression::fast()))
}

pub fn gzip_stream_decompress(input: impl Stream<Item = io::Result<Bytes>>) -> Vec<u8> {
    use async_compression::stream::GzipDecoder;
    pin_mut!(input);
    stream_to_vec(GzipDecoder::new(input))
}
