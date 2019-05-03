use bytes::Bytes;
use flate2::bufread::GzDecoder;
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
fn decompressed_gzip_stream_1() {
    use async_compression::stream::gzip;

    let stream = stream::iter(vec![
        Bytes::from_static(&[1, 2, 3]),
        Bytes::from_static(&[4, 5, 6]),
    ]);
    let compressed = gzip::GzipStream::new(stream.map(Ok), gzip::Compression::default());
    let decompressed = gzip::DecompressedGzipStream::new(compressed);
    let data: Vec<_> = block_on(decompressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    assert_eq!(data, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn decompressed_gzip_stream_2() {
    use async_compression::stream::gzip;

    let stream = stream::iter(vec![
        Bytes::from_static(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
        Bytes::from_static(&[11, 12, 13, 14, 15]),
    ]);
    let compressed = gzip::GzipStream::new(stream.map(Ok), gzip::Compression::default());
    let decompressed = gzip::DecompressedGzipStream::new(compressed);
    let data: Vec<_> = block_on(decompressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    assert_eq!(
        data,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
    );
}

#[test]
fn decompressed_gzip_stream_3() {
    use async_compression::stream::gzip;

    let stream = stream::iter(vec![Bytes::from_static(&[])]);
    let compressed = gzip::GzipStream::new(stream.map(Ok), gzip::Compression::default());
    let decompressed = gzip::DecompressedGzipStream::new(compressed);
    let data: Vec<_> = block_on(decompressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    assert_eq!(data, vec![]);
}
