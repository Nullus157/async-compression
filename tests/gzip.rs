use bytes::Bytes;
use flate2::bufread::GzDecoder;
use futures::{
    executor::block_on,
    stream::{self, StreamExt},
};
use std::io::{self, Read};

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
