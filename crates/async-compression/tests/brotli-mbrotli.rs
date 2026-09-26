#![cfg(async_compression_unstable)]

use compression_codecs::mbrotli::params::EncoderParams;

#[macro_use]
mod utils;

test_cases!(mbrotli);

#[test]
pub fn mbrotli_params() {
    let _ = EncoderParams::default();
}

#[cfg(feature = "tokio")]
mod params {
    use async_compression::{mbrotli::EncoderParams, tokio::bufread::MbrotliEncoder, Level};
    use tokio::io::AsyncReadExt as _;

    use crate::utils::{algos::mbrotli::sync, block_on};

    fn input() -> Vec<u8> {
        b"the quick brown fox jumps over the lazy dog\n".repeat(1024)
    }

    fn compress(input: &[u8], params: EncoderParams) -> Vec<u8> {
        let mut encoder = MbrotliEncoder::with_params(input, params);
        let mut output = Vec::new();
        block_on(encoder.read_to_end(&mut output)).unwrap();
        output
    }

    #[test]
    #[ntest::timeout(10000)]
    fn every_quality_roundtrips() {
        let input = input();
        for quality in -1..=12 {
            let params = EncoderParams::default().quality(Level::Precise(quality));
            let compressed = compress(&input, params);
            assert_eq!(sync::decompress(&compressed), input, "quality {quality}");
        }
    }

    #[test]
    #[ntest::timeout(10000)]
    fn zero_window_size_selects_default_window() {
        let input = input();
        let default = compress(&input, EncoderParams::default());
        let zero = compress(&input, EncoderParams::default().window_size(0));
        let smallest = compress(&input, EncoderParams::default().window_size(1));

        assert_eq!(zero, default);
        assert_ne!(smallest, default);
    }

    #[test]
    #[ntest::timeout(10000)]
    fn tuned_params_roundtrip() {
        let input = input();
        let params = EncoderParams::default()
            .quality(Level::Default)
            .window_size(0)
            .block_size(100)
            .size_hint(input.len())
            .text_mode();
        let compressed = compress(&input, params);
        assert_eq!(sync::decompress(&compressed), input);

        let params = EncoderParams::default()
            .quality(Level::Fastest)
            .window_size(100)
            .font_mode();
        let compressed = compress(&input, params);
        assert_eq!(sync::decompress(&compressed), input);
    }
}
