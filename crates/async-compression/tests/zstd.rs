#[macro_use]
mod utils;

test_cases!(zstd);

#[cfg(feature = "tokio")]
#[test]
#[ntest::timeout(1000)]
fn bufread_multiple_members_with_corrupt_second_frame() {
    use async_compression::tokio::bufread::ZstdDecoder;
    use tokio::io::{sink, BufReader};
    use utils::algos::zstd::sync;

    let first = sync::compress(&[1, 2, 3, 4, 5, 6]);
    let mut second = sync::compress(&[0; 2048]);
    let corrupt = second.len() - 2;
    second[corrupt] ^= 0xff;
    let compressed = [first, second].join(&[][..]);

    let mut decoder = ZstdDecoder::new(BufReader::new(compressed.as_slice()));
    decoder.multiple_members(true);

    let result = futures::executor::block_on(tokio::io::copy(&mut decoder, &mut sink()));
    assert!(result.is_err());
}
