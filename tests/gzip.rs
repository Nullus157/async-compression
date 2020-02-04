use std::iter::FromIterator;

use bytes::Bytes;
use proptest::{prelude::any, proptest};

#[macro_use]
mod utils;

test_cases!(gzip);

/// Splits the input bytes into the first 10 bytes, the rest and the last 8 bytes, taking apart the
/// 3 parts of compressed gzip data.
fn split(mut input: Vec<u8>) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    assert!(input.len() >= 18);

    let mut body = input.split_off(10);
    let header = input;
    let footer = body.split_off(body.len() - 8);

    (header, body, footer)
}

#[test]
#[ntest::timeout(1000)]
fn gzip_stream_decompress_single_chunk() {
    let compressed = utils::gzip::sync::compress(&[1, 2, 3, 4, 5, 6]);

    // The entirety in one chunk
    let stream = utils::InputStream::from(vec![compressed]);
    let output = utils::gzip::stream::decompress(stream.stream());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
fn gzip_stream_decompress_segmented() {
    let (header, body, footer) = split(utils::gzip::sync::compress(&[1, 2, 3, 4, 5, 6]));

    // Header, body and footer in separate chunks, similar to how `GzipStream` outputs it.
    let stream = utils::InputStream::from(vec![header, body, footer]);
    let output = utils::gzip::stream::decompress(stream.stream());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
fn gzip_stream_decompress_split() {
    let (header, body, footer) = split(utils::gzip::sync::compress(&[1, 2, 3, 4, 5, 6]));

    // Header, body and footer each split across multiple chunks, no mixing
    let stream = utils::InputStream::from(vec![
        Vec::from(&header[0..5]),
        Vec::from(&header[5..10]),
        Vec::from(&body[0..body.len() / 2]),
        Vec::from(&body[body.len() / 2..]),
        Vec::from(&footer[0..4]),
        Vec::from(&footer[4..8]),
    ]);

    let output = utils::gzip::stream::decompress(stream.stream());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
fn gzip_stream_decompress_split_mixed() {
    let (header, body, footer) = split(utils::gzip::sync::compress(&[1, 2, 3, 4, 5, 6]));

    // Header, body and footer split across multiple chunks and mixed together
    let stream = utils::InputStream::from(vec![
        Vec::from(&header[0..5]),
        Vec::from_iter(
            header[5..10]
                .iter()
                .chain(body[0..body.len() / 2].iter())
                .cloned(),
        ),
        Vec::from_iter(
            body[body.len() / 2..]
                .iter()
                .chain(footer[0..4].iter())
                .cloned(),
        ),
        Vec::from(&footer[4..8]),
    ]);

    let output = utils::gzip::stream::decompress(stream.stream());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

fn compress_with_header(data: &[u8]) -> Vec<u8> {
    use flate2::{Compression, GzBuilder};
    use std::io::Write;

    let mut bytes = Vec::new();
    {
        let mut gz = GzBuilder::new()
            .filename("hello_world.txt")
            .comment("test file, please delete")
            .write(&mut bytes, Compression::fast());

        gz.write_all(data).unwrap();
    }

    bytes
}

#[test]
#[ntest::timeout(1000)]
fn gzip_stream_decompress_with_extra_header() {
    let bytes = compress_with_header(&[1, 2, 3, 4, 5, 6]);

    let stream = utils::InputStream::from(vec![bytes]);
    let output = utils::gzip::stream::decompress(stream.stream());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
fn gzip_stream_chunks_decompress_with_extra_header() {
    let bytes = compress_with_header(&[1, 2, 3, 4, 5, 6]);

    let stream = utils::InputStream::from(bytes.chunks(2).map(Vec::from).collect::<Vec<_>>());
    let output = utils::gzip::stream::decompress(stream.stream());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
fn gzip_bufread_decompress_with_extra_header() {
    let bytes = compress_with_header(&[1, 2, 3, 4, 5, 6]);

    let stream = utils::InputStream::from(vec![bytes]);
    let output = utils::gzip::futures::bufread::decompress(stream.reader());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
fn gzip_bufread_decompress_concatenated() {
    let bytes1 = compress_with_header(&[1, 2]);
    let bytes2 = compress_with_header(&[3, 4]);
    let bytes3 = compress_with_header(&[5, 6]);

    let mut concatenated = Vec::new();
    concatenated.extend_from_slice(&bytes1);
    concatenated.extend_from_slice(&bytes2);
    concatenated.extend_from_slice(&bytes3);

    let stream = utils::InputStream::from(vec![concatenated]);
    let output = utils::gzip::futures::bufread::decompress(stream.reader());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

proptest! {
  #[test]
  fn gzip_stream_decompress_concatenated(
    ref data in any::<Vec<u8>>(),
    iterations in 1..100usize,
    chunk_size in 1..1024usize,
  ) {
    proptest_gzip_stream_decompress_concatenated(data.clone(), iterations, chunk_size)
  }
}

fn proptest_gzip_stream_decompress_concatenated(
    data: Vec<u8>,
    iterations: usize,
    chunk_size: usize,
) {
    use futures::stream::{iter, StreamExt};

    let payload: Vec<u8> = (0..iterations).map(|_| data.clone()).flatten().collect();

    let byte_stream = iter(payload.clone()).chunks(chunk_size).map(|bytes| {
        let compressed = compress_with_header(&bytes);
        let res: std::io::Result<Bytes> = Ok(Bytes::from(compressed));
        res
    });

    let output = utils::gzip::stream::decompress(byte_stream);

    assert_eq!(output, payload);
}

#[test]
#[ntest::timeout(1000)]
fn gzip_bufread_chunks_decompress_with_extra_header() {
    let bytes = compress_with_header(&[1, 2, 3, 4, 5, 6]);

    let stream = utils::InputStream::from(bytes.chunks(2).map(Vec::from).collect::<Vec<_>>());
    let output = utils::gzip::futures::bufread::decompress(stream.reader());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}
