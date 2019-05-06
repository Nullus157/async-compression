use std::iter::FromIterator;

mod utils;

#[test]
fn deflate_stream_compress() {
    let input = utils::InputStream::from([[1, 2, 3], [4, 5, 6]]);

    let compressed = utils::deflate_stream_compress(input.stream());
    let output = utils::deflate_decompress(&compressed);

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn deflate_stream_compress_large() {
    let input = vec![
        Vec::from_iter((0..20_000).map(|_| rand::random())),
        Vec::from_iter((0..20_000).map(|_| rand::random())),
    ];
    let input = utils::InputStream::from(input);

    let compressed = utils::deflate_stream_compress(input.stream());
    let output = utils::deflate_decompress(&compressed);

    assert_eq!(output, input.bytes());
}

#[test]
fn deflate_stream_decompress() {
    let compressed = utils::deflate_compress(&[1, 2, 3, 4, 5, 6][..]);

    let stream = utils::InputStream::from(vec![compressed]);
    let output = utils::deflate_stream_decompress(stream.stream());

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn deflate_read_compress() {
    let input = utils::InputStream::from([[1, 2, 3], [4, 5, 6]]);

    let compressed = utils::deflate_read_compress(input.reader());
    let output = utils::deflate_decompress(&compressed);

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}
