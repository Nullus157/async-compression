#[allow(unused)]
use std::iter::FromIterator;

#[macro_use]
mod utils;

test_cases!(gzip);

#[allow(unused)]
use utils::{algos::gzip::sync, InputStream};

#[cfg(feature = "stream")]
use utils::algos::gzip::stream;

#[cfg(feature = "futures-io")]
use utils::algos::gzip::futures::bufread;

/// Splits the input bytes into the first 10 bytes, the rest and the last 8 bytes, taking apart the
/// 3 parts of compressed gzip data.
#[allow(unused)]
fn split(mut input: Vec<u8>) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    assert!(input.len() >= 18);

    let mut body = input.split_off(10);
    let header = input;
    let footer = body.split_off(body.len() - 8);

    (header, body, footer)
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "stream")]
fn gzip_stream_decompress_single_chunk() {
    let compressed = sync::compress(&[1, 2, 3, 4, 5, 6]);

    // The entirety in one chunk
    let input = InputStream::from(vec![compressed]);
    let output = stream::decompress(input.bytes_05_stream());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "stream")]
fn gzip_stream_decompress_segmented() {
    let (header, body, footer) = split(sync::compress(&[1, 2, 3, 4, 5, 6]));

    // Header, body and footer in separate chunks, similar to how `GzipStream` outputs it.
    let input = InputStream::from(vec![header, body, footer]);
    let output = stream::decompress(input.bytes_05_stream());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "stream")]
fn gzip_stream_decompress_split() {
    let (header, body, footer) = split(sync::compress(&[1, 2, 3, 4, 5, 6]));

    // Header, body and footer each split across multiple chunks, no mixing
    let input = InputStream::from(vec![
        Vec::from(&header[0..5]),
        Vec::from(&header[5..10]),
        Vec::from(&body[0..body.len() / 2]),
        Vec::from(&body[body.len() / 2..]),
        Vec::from(&footer[0..4]),
        Vec::from(&footer[4..8]),
    ]);

    let output = stream::decompress(input.bytes_05_stream());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "stream")]
fn gzip_stream_decompress_split_mixed() {
    let (header, body, footer) = split(sync::compress(&[1, 2, 3, 4, 5, 6]));

    // Header, body and footer split across multiple chunks and mixed together
    let input = InputStream::from(vec![
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

    let output = stream::decompress(input.bytes_05_stream());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[allow(unused)]
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
#[cfg(feature = "stream")]
fn gzip_stream_decompress_with_extra_header() {
    let bytes = compress_with_header(&[1, 2, 3, 4, 5, 6]);

    let input = InputStream::from(vec![bytes]);
    let output = stream::decompress(input.bytes_05_stream());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "stream")]
fn gzip_stream_chunks_decompress_with_extra_header() {
    let bytes = compress_with_header(&[1, 2, 3, 4, 5, 6]);

    let input = InputStream::from(bytes.chunks(2).map(Vec::from).collect::<Vec<_>>());
    let output = stream::decompress(input.bytes_05_stream());

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "futures-io")]
fn gzip_bufread_decompress_with_extra_header() {
    let bytes = compress_with_header(&[1, 2, 3, 4, 5, 6]);

    let input = InputStream::from(vec![bytes]);
    let output = bufread::decompress(bufread::from(&input));

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "futures-io")]
fn gzip_bufread_chunks_decompress_with_extra_header() {
    let bytes = compress_with_header(&[1, 2, 3, 4, 5, 6]);

    let input = InputStream::from(bytes.chunks(2).map(Vec::from).collect::<Vec<_>>());
    let output = bufread::decompress(bufread::from(&input));

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}
