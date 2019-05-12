use std::iter::FromIterator;

mod utils;

#[test]
fn zstd_stream_compress() {
    let input = utils::InputStream::from([[1, 2, 3], [4, 5, 6]]);

    let compressed = utils::zstd_stream_compress(input.stream());
    let output = utils::zstd_decompress(&compressed);

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn zstd_stream_compress_large() {
    let input = vec![
        Vec::from_iter((0..20_000).map(|_| rand::random())),
        Vec::from_iter((0..20_000).map(|_| rand::random())),
    ];
    let input = utils::InputStream::from(input);

    let compressed = utils::zstd_stream_compress(input.stream());
    let output = utils::zstd_decompress(&compressed);

    assert_eq!(output, input.bytes());
}

#[test]
fn zstd_stream_decompress() {
    let compressed = utils::zstd_compress(&[1, 2, 3, 4, 5, 6][..]);

    let stream = utils::InputStream::from(vec![compressed]);
    let output = utils::zstd_stream_decompress(stream.stream());

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}
