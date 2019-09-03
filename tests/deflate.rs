use std::iter::FromIterator;

#[macro_use]
mod utils;

test_cases!(deflate);

#[test]
fn deflate_bufread_compress() {
    let input = utils::InputStream::from([[1, 2, 3], [4, 5, 6]]);

    let compressed = utils::deflate::bufread::compress(input.reader());
    let output = utils::deflate::sync::decompress(&compressed);

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn deflate_bufread_compress_large() {
    let input = vec![
        Vec::from_iter((0..20_000).map(|_| rand::random())),
        Vec::from_iter((0..20_000).map(|_| rand::random())),
    ];
    let input = utils::InputStream::from(input);

    let compressed = utils::deflate::bufread::compress(input.reader());
    let output = utils::deflate::sync::decompress(&compressed);

    assert_eq!(output, input.bytes());
}
