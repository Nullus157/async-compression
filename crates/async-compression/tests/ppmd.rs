mod utils;

use utils::{one_to_six, one_to_six_stream, InputStream};

#[cfg(feature = "futures-io")]
mod futures_tests {
    use super::*;
    use utils::{
        algos::ppmd::{
            futures::{bufread, read},
            sync,
        },
        Level,
    };

    #[test]
    fn compress_decompress_small() {
        let compressed = bufread::compress(bufread::from(&one_to_six_stream()));
        let output = sync::decompress(&compressed);
        assert_eq!(output, one_to_six());
    }

    #[test]
    fn compress_with_levels() {
        let encoder =
            bufread::Encoder::with_quality(bufread::from(&one_to_six_stream()), Level::Best);
        let compressed = read::to_vec(encoder);
        let output = sync::decompress(&compressed);
        assert_eq!(output, one_to_six());
    }
}

#[cfg(feature = "tokio")]
mod tokio_tests {
    use super::*;
    use utils::algos::ppmd::{sync, tokio::bufread};

    #[test]
    fn compress_decompress_random() {
        let input = InputStream::new(vec![
            (0..16_384).map(|_| rand::random()).collect(),
            (0..16_384).map(|_| rand::random()).collect(),
        ]);
        let compressed = bufread::compress(bufread::from(&input));
        let output = sync::decompress(&compressed);
        assert_eq!(output, input.bytes());
    }

    #[test]
    fn write_side_compress_decompress() {
        use utils::algos::ppmd::tokio::write;
        let input = one_to_six_stream();
        let compressed = write::compress(input.as_ref(), 65_536);
        let output = sync::decompress(&compressed);
        assert_eq!(output, one_to_six());
    }
}
