#[macro_use]
mod utils;

test_cases!(gzip);

#[allow(unused)]
use utils::{algos::gzip::sync, InputStream};

#[cfg(feature = "futures-io")]
use utils::algos::gzip::futures::bufread;

#[allow(unused)]
fn compress_with_header(data: &[u8]) -> Vec<u8> {
    use flate2::{Compression, GzBuilder};
    use std::io::Write;

    let mut bytes = Vec::new();
    {
        let mut gz = GzBuilder::new()
            .filename("hello_world.txt")
            .comment("test file, please delete")
            .extra(vec![1, 2, 3, 4])
            .write(&mut bytes, Compression::fast());

        gz.write_all(data).unwrap();
    }

    bytes
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "futures-io")]
fn gzip_bufread_decompress_with_extra_header() {
    let bytes = compress_with_header(&[1, 2, 3, 4, 5, 6]);

    let input = InputStream::from(vec![bytes]);
    let output = bufread::decompress(bufread::from(&input));

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "futures-io")]
fn gzip_bufread_chunks_decompress_with_extra_header() {
    let bytes = compress_with_header(&[1, 2, 3, 4, 5, 6]);

    let input = InputStream::from(bytes.chunks(2));
    let output = bufread::decompress(bufread::from(&input));

    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "futures-io")]
fn gzip_bufread_chunks_decompress_without_footer_emits_all_payload() {
    use flate2::bufread::GzDecoder;
    use std::io::Read;

    let mut bytes = compress_with_header(&[1, 2, 3, 4, 5, 6]);

    // Remove the footer.
    bytes.truncate(bytes.len() - 8);

    let mut decoder = GzDecoder::new(bytes.as_slice());

    let mut output = vec![];
    let result = decoder.read_to_end(&mut output);

    assert!(result.is_err());
    assert_eq!(output, &[1, 2, 3, 4, 5, 6][..]);
}
