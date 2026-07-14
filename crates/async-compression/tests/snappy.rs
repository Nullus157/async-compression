#[macro_use]
mod utils;

use crate::utils::algos::snappy::sync;
use crate::utils::algos::snappy::tokio;
use crate::utils::InputStream;
use proptest::{collection::vec as prop_vec, prelude::*};
use std::iter::FromIterator;

test_cases!(snappy);

/// A random short seed repeated a random number of times to create a compressible payload
fn compressible_bytes() -> impl Strategy<Value = Vec<u8>> {
    (prop_vec(any::<u8>(), 1..=32), 1usize..=8192)
        .prop_map(|(seed, n)| seed.iter().copied().cycle().take(seed.len() * n).collect())
}

/// The same payload, sliced into an `InputStream` with a random chunk size
fn compressible_input_stream() -> impl Strategy<Value = InputStream> {
    (compressible_bytes(), 1usize..=16_384).prop_map(|(bytes, chunk)| {
        InputStream::new(bytes.chunks(chunk).map(<[u8]>::to_vec).collect())
    })
}

proptest! {

    #[test]
    fn write_compress_compressible_input(ref input in compressible_input_stream(), limit in 1..20usize) {
        let compressed = tokio::write::compress(input.as_ref(), limit);
        let output = sync::decompress(&compressed);
        assert_eq!(output, input.bytes())
    }

    #[test]
    fn bufread_compress_compressible_input(ref input in compressible_input_stream()) {
        let buf = tokio::bufread::from(input);
        let compressed = tokio::bufread::compress(buf);
        let output = sync::decompress(&compressed);
        assert_eq!(output, input.bytes())
    }

    #[test]
    fn decompressed_compressible_input(ref bytes in compressible_bytes(), chunk_size in 1..20usize) {
        let compressed = sync::compress(bytes);
        let input = InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
        let output = tokio::bufread::decompress(tokio::bufread::from(&input));
        assert_eq!(&output, bytes);
    }
}
