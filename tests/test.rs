use std::io::{self, Read};
use bytes::Bytes;
use futures::{stream::{self, StreamExt}, executor::block_on, io::AsyncReadExt};
use flate2::bufread::DeflateDecoder;

#[test]
fn stream() {
    let stream = stream::iter(vec![Bytes::from_static(&[1, 2, 3]), Bytes::from_static(&[4, 5, 6])]);
    let compressed = async_compression::compress_stream(stream.map(Ok));
    let data: Vec<_> = block_on(compressed.collect());
    let data: io::Result<Vec<_>> = data.into_iter().collect();
    let data: Vec<u8> = data.unwrap().into_iter().flatten().collect();
    let mut output = vec![];
    DeflateDecoder::new(&data[..]).read_to_end(&mut output).unwrap();
    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn read() {
    let input = &[1, 2, 3, 4, 5, 6];
    let mut compressed = async_compression::compress_read(&input[..]);
    let mut data = vec![];
    block_on(compressed.read_to_end(&mut data)).unwrap();
    let mut output = vec![];
    DeflateDecoder::new(&data[..]).read_to_end(&mut output).unwrap();
    assert_eq!(output, input);
}
