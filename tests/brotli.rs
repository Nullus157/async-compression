use brotli2::bufread::BrotliDecoder;
use bytes::Bytes;
use futures::{
    executor::block_on,
    stream::{self, StreamExt},
};
use std::io::{self, Read};
use std::iter::FromIterator;

#[test]
fn brotli_stream() {
    use async_compression::stream::brotli;

    let stream = stream::iter(vec![
        Bytes::from_static(&[1, 2, 3]),
        Bytes::from_static(&[4, 5, 6]),
    ]);
    let compress = brotli::Compress::new();
    let compressed = brotli::BrotliStream::new(stream.map(Ok), compress);
    let data: Vec<_> = block_on(compressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    let mut output = vec![];
    BrotliDecoder::new(&data[..])
        .read_to_end(&mut output)
        .unwrap();
    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn brotli_stream_large() {
    use async_compression::stream::brotli;

    let bytes = [
        Vec::from_iter((0..20_000).map(|_| rand::random())),
        Vec::from_iter((0..20_000).map(|_| rand::random())),
    ];

    let stream = stream::iter(vec![
        Bytes::from(bytes[0].clone()),
        Bytes::from(bytes[1].clone()),
    ]);
    let compress = brotli::Compress::new();
    let compressed = brotli::BrotliStream::new(stream.map(Ok), compress);
    let data: Vec<_> = block_on(compressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    let mut output = vec![];
    BrotliDecoder::new(&data[..])
        .read_to_end(&mut output)
        .unwrap();
    assert_eq!(
        output,
        Vec::from_iter(bytes[0].iter().chain(bytes[1].iter()).cloned())
    );
}

#[test]
fn decompressed_brotli_stream() {
    use async_compression::stream::brotli;

    let stream = stream::iter(vec![
        Bytes::from_static(&[1, 2, 3]),
        Bytes::from_static(&[4, 5, 6]),
    ]);
    let compress = brotli::Compress::new();
    let compressed = brotli::BrotliStream::new(stream.map(Ok), compress);
    let decompressed = brotli::DecompressedBrotliStream::new(compressed);
    let data: Vec<_> = block_on(decompressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    assert_eq!(data, vec![1, 2, 3, 4, 5, 6]);
}
