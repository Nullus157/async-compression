use proptest::{prelude::any, proptest};
use std::iter::FromIterator;

mod utils;

proptest! {
    #[test]
    fn brotli_stream_compress(ref input in any::<utils::InputStream>()) {
        let compressed = utils::brotli_stream_compress(input.stream());
        let output = utils::brotli_decompress(&compressed);
        assert_eq!(output, input.bytes());
    }

    #[test]
    fn brotli_stream_decompress(
        ref input in any::<Vec<u8>>(),
        chunk_size in 1..20usize,
    ) {
        let compressed = utils::brotli_compress(input);
        let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
        let output = utils::brotli_stream_decompress(stream.stream());
        assert_eq!(&output, input);
    }

    #[test]
    fn deflate_stream_compress(ref input in any::<utils::InputStream>()) {
        let compressed = utils::deflate_stream_compress(input.stream());
        let output = utils::deflate_decompress(&compressed);
        assert_eq!(output, input.bytes());
    }

    #[test]
    fn deflate_stream_decompress(
        ref input in any::<Vec<u8>>(),
        chunk_size in 1..20usize,
    ) {
        let compressed = utils::deflate_compress(input);
        let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
        let output = utils::deflate_stream_decompress(stream.stream());
        assert_eq!(&output, input);
    }

    #[test]
    fn deflate_bufread_compress(ref input in any::<utils::InputStream>()) {
        let compressed = utils::deflate_bufread_compress(input.reader());
        let output = utils::deflate_decompress(&compressed);
        assert_eq!(output, input.bytes());
    }

    #[test]
    fn zlib_stream_compress(ref input in any::<utils::InputStream>()) {
        let compressed = utils::zlib_stream_compress(input.stream());
        let output = utils::zlib_decompress(&compressed);
        assert_eq!(output, input.bytes());
    }

    #[test]
    fn zlib_stream_decompress(
        ref input in any::<Vec<u8>>(),
        chunk_size in 1..20usize,
    ) {
        let compressed = utils::zlib_compress(input);
        let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
        let output = utils::zlib_stream_decompress(stream.stream());
        assert_eq!(&output, input);
    }

    #[test]
    fn zlib_bufread_compress(ref input in any::<utils::InputStream>()) {
        let compressed = utils::zlib_bufread_compress(input.reader());
        let output = utils::zlib_decompress(&compressed);
        assert_eq!(output, input.bytes());
    }

    #[test]
    fn gzip_stream_compress(ref input in any::<utils::InputStream>()) {
        let compressed = utils::gzip_stream_compress(input.stream());
        let output = utils::gzip_decompress(&compressed);
        assert_eq!(output, input.bytes());
    }

    #[test]
    fn gzip_stream_decompress(
        ref input in any::<Vec<u8>>(),
        chunk_size in 1..20usize,
    ) {
        let compressed = utils::gzip_compress(input);
        let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
        let output = utils::gzip_stream_decompress(stream.stream());
        assert_eq!(&output, input);
    }
}
