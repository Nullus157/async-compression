use bytes::Bytes;
use flate2::bufread::DeflateDecoder;
use futures::{
    executor::block_on,
    io::AsyncReadExt,
    stream::{self, StreamExt},
};
use std::io::{self, Read};

#[test]
fn flate2_stream() {
    use async_compression::flate2;

    let stream = stream::iter(vec![
        Bytes::from_static(&[1, 2, 3]),
        Bytes::from_static(&[4, 5, 6]),
    ]);
    let compress = flate2::Compress::new(flate2::Compression::default(), false);
    let compressed = flate2::compress_stream(stream.map(Ok), compress);
    let data: Vec<_> = block_on(compressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    let mut output = vec![];
    DeflateDecoder::new(&data[..])
        .read_to_end(&mut output)
        .unwrap();
    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn flate2_read() {
    use async_compression::flate2;

    let input = &[1, 2, 3, 4, 5, 6];
    let compress = flate2::Compress::new(flate2::Compression::default(), false);
    let mut compressed = flate2::compress_read(&input[..], compress);
    let mut data = vec![];
    block_on(compressed.read_to_end(&mut data)).unwrap();
    let mut output = vec![];
    DeflateDecoder::new(&data[..])
        .read_to_end(&mut output)
        .unwrap();
    assert_eq!(output, input);
}
