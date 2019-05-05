use bytes::{BufMut, Bytes, IntoBuf};
use flate2::bufread::{GzDecoder, GzEncoder};
use futures::{
    executor::block_on,
    stream::{self, StreamExt},
};
use std::io::{self, Read};
use std::iter::FromIterator;

#[test]
fn gzip_stream() {
    use async_compression::stream::gzip;

    let stream = stream::iter(vec![
        Bytes::from_static(&[1, 2, 3]),
        Bytes::from_static(&[4, 5, 6]),
    ]);
    let compressed = gzip::GzipStream::new(stream.map(Ok), gzip::Compression::default());
    let data: Vec<_> = block_on(compressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    let mut output = vec![];
    GzDecoder::new(&data[..]).read_to_end(&mut output).unwrap();
    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn gzip_stream_large() {
    use async_compression::stream::gzip;

    let bytes = [
        Vec::from_iter((0..20_000).map(|_| rand::random())),
        Vec::from_iter((0..20_000).map(|_| rand::random())),
    ];

    let stream = stream::iter(vec![
        Bytes::from(bytes[0].clone()),
        Bytes::from(bytes[1].clone()),
    ]);
    let compressed = gzip::GzipStream::new(stream.map(Ok), gzip::Compression::default());
    let data: Vec<_> = block_on(compressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    let mut output = vec![];
    GzDecoder::new(&data[..]).read_to_end(&mut output).unwrap();
    assert_eq!(
        output,
        Vec::from_iter(bytes[0].iter().chain(bytes[1].iter()).cloned())
    );
}

#[test]
fn decompressed_gzip_stream_single_chunk() {
    use async_compression::stream::gzip;

    let bytes = Bytes::from_static(&[1, 2, 3, 4, 5, 6]).into_buf();

    let mut gz = GzEncoder::new(bytes, gzip::Compression::default());
    let mut buffer = Vec::new();

    gz.read_to_end(&mut buffer).unwrap();

    // The entirety in one chunk
    let stream = stream::iter(vec![Bytes::from(buffer)]);

    let decompressed = gzip::DecompressedGzipStream::new(stream.map(Ok));
    let data: Vec<_> = block_on(decompressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    assert_eq!(data, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn decompressed_gzip_stream_segmented() {
    use async_compression::stream::gzip;

    let bytes = Bytes::from_static(&[1, 2, 3, 4, 5, 6]).into_buf();

    let mut gz = GzEncoder::new(bytes, gzip::Compression::default());
    let mut buffer = Vec::new();

    gz.read_to_end(&mut buffer).unwrap();

    let body_end = buffer.len() - 8;

    let header = &buffer[..10];
    let body = &buffer[10..body_end];
    let footer = &buffer[body_end..];

    // Header, body and trailer in separate chunks, similar to how `GzipStream` outputs it.
    let stream = stream::iter(vec![
        Bytes::from(&header[..]),
        Bytes::from(&body[..]),
        Bytes::from(&footer[..]),
    ]);

    let decompressed = gzip::DecompressedGzipStream::new(stream.map(Ok));
    let data: Vec<_> = block_on(decompressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    assert_eq!(data, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn decompressed_gzip_stream_split() {
    use async_compression::stream::gzip;

    let bytes = Bytes::from_static(&[1, 2, 3, 4, 5, 6]).into_buf();

    let mut gz = GzEncoder::new(bytes, gzip::Compression::default());
    let mut buffer = Vec::new();

    gz.read_to_end(&mut buffer).unwrap();

    let body_end = buffer.len() - 8;

    let header = &buffer[..10];
    let body = &buffer[10..body_end];
    let footer = &buffer[body_end..];

    let body_half = body.len() / 2;

    // Header, body and trailer split across multiple chunks and mixed together
    let stream = stream::iter(vec![
        Bytes::from(&header[0..5]),
        Bytes::from(Vec::from_iter(
            header[5..10]
                .iter()
                .chain(body[0..body_half].iter())
                .cloned(),
        )),
        Bytes::from(Vec::from_iter(
            body[body_half..body_end]
                .iter()
                .chain(footer[0..4].iter())
                .cloned(),
        )),
        Bytes::from(&footer[4..8]),
    ]);

    let decompressed = gzip::DecompressedGzipStream::new(stream.map(Ok));
    let data: Vec<_> = block_on(decompressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    assert_eq!(data, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn decompressed_gzip_stream_split_mixed() {
    use async_compression::stream::gzip;

    let bytes = Bytes::from_static(&[1, 2, 3, 4, 5, 6]).into_buf();

    let mut gz = GzEncoder::new(bytes, gzip::Compression::default());
    let mut buffer = Vec::new();

    gz.read_to_end(&mut buffer).unwrap();

    let body_end = buffer.len() - 8;

    let header = &buffer[..10];
    let body = &buffer[10..body_end];
    let footer = &buffer[body_end..];

    let body_half = body.len() / 2;

    // Header, body and trailer each split across multiple chunks, no mixing
    let stream = stream::iter(vec![
        Bytes::from(&header[0..5]),
        Bytes::from(&header[5..10]),
        Bytes::from(&body[0..body_half]),
        Bytes::from(&body[body_half..body_end]),
        Bytes::from(&footer[0..4]),
        Bytes::from(&footer[4..8]),
    ]);

    let decompressed = gzip::DecompressedGzipStream::new(stream.map(Ok));
    let data: Vec<_> = block_on(decompressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    assert_eq!(data, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn decompressed_gzip_stream_empty() {
    use async_compression::stream::gzip;

    let bytes = Bytes::from_static(&[]).into_buf();

    let mut gz = GzEncoder::new(bytes, gzip::Compression::default());
    let mut buffer = Vec::new();

    gz.read_to_end(&mut buffer).unwrap();

    let header = &buffer[..10];
    let body = &buffer[10..buffer.len() - 8];
    let footer = &buffer[buffer.len() - 8..];

    // Header, body and trailer in separate chunks, similar to how `GzipStream` outputs it.
    let stream = stream::iter(vec![
        Bytes::from(&header[..]),
        Bytes::from(&body[..]),
        Bytes::from(&footer[..]),
    ]);

    let decompressed = gzip::DecompressedGzipStream::new(stream.map(Ok));
    let data: Vec<_> = block_on(decompressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    assert_eq!(data, vec![]);
}
