use bytes::{BufMut, Bytes, IntoBuf};
use flate2::bufread::{ZlibDecoder, ZlibEncoder};
use futures::{
    executor::block_on,
    io::AsyncReadExt,
    stream::{self, StreamExt},
};
use std::io::{self, Read};
use std::iter::FromIterator;

#[test]
fn zlib_stream() {
    use async_compression::stream::zlib;

    let stream = stream::iter(vec![
        Bytes::from_static(&[1, 2, 3]),
        Bytes::from_static(&[4, 5, 6]),
    ]);
    let compressed = zlib::ZlibStream::new(stream.map(Ok), zlib::Compression::default());
    let data: Vec<_> = block_on(compressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    let mut output = vec![];
    ZlibDecoder::new(&data[..])
        .read_to_end(&mut output)
        .unwrap();
    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn zlib_stream_large() {
    use async_compression::stream::zlib;

    let bytes = [
        Vec::from_iter((0..20_000).map(|_| rand::random())),
        Vec::from_iter((0..20_000).map(|_| rand::random())),
    ];

    let stream = stream::iter(vec![
        Bytes::from(bytes[0].clone()),
        Bytes::from(bytes[1].clone()),
    ]);
    let compressed = zlib::ZlibStream::new(stream.map(Ok), zlib::Compression::default());
    let data: Vec<_> = block_on(compressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    let mut output = vec![];
    ZlibDecoder::new(&data[..])
        .read_to_end(&mut output)
        .unwrap();
    assert_eq!(
        output,
        Vec::from_iter(bytes[0].iter().chain(bytes[1].iter()).cloned())
    );
}

#[test]
fn decompressed_zlib_stream() {
    use async_compression::stream::zlib;

    let bytes = Bytes::from_static(&[1, 2, 3, 4, 5, 6]).into_buf();

    let mut gz = ZlibEncoder::new(bytes, zlib::Compression::default());
    let mut buffer = Vec::new();

    gz.read_to_end(&mut buffer).unwrap();

    let stream = stream::iter(vec![Bytes::from(buffer)]);
    let decompressed = zlib::DecompressedZlibStream::new(stream.map(Ok));
    let data: Vec<_> = block_on(decompressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();

    assert_eq!(data, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn zlib_read() {
    use async_compression::read::zlib;

    let input = &[1, 2, 3, 4, 5, 6];
    let mut compressed = zlib::ZlibRead::new(&input[..], zlib::Compression::default());
    let mut data = vec![];
    block_on(compressed.read_to_end(&mut data)).unwrap();
    let mut output = vec![];
    ZlibDecoder::new(&data[..])
        .read_to_end(&mut output)
        .unwrap();
    assert_eq!(output, input);
}
