#[macro_use]
mod utils;

test_cases!(zlib);

#[test]
fn zlib_bufread_compress() {
    let input = utils::InputStream::from([[1, 2, 3], [4, 5, 6]]);

    let compressed = utils::zlib::bufread::compress(input.reader());
    let output = utils::zlib::sync::decompress(&compressed);

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}
