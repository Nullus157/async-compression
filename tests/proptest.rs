mod utils;

mod brotli {
    mod stream {
        use crate::utils;
        use proptest::{prelude::any, proptest};
        use std::iter::FromIterator;
        proptest! {
            #[test]
            fn compress(ref input in any::<utils::InputStream>()) {
                let compressed = utils::brotli::stream::compress(input.stream());
                let output = utils::brotli::sync::decompress(&compressed);
                assert_eq!(output, input.bytes());
            }

            #[test]
            fn decompress(
                ref input in any::<Vec<u8>>(),
                chunk_size in 1..20usize,
            ) {
                let compressed = utils::brotli::sync::compress(input);
                let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
                let output = utils::brotli::stream::decompress(stream.stream());
                assert_eq!(&output, input);
            }
        }
    }
}

mod deflate {
    mod stream {
        use crate::utils;
        use proptest::{prelude::any, proptest};
        use std::iter::FromIterator;
        proptest! {
            #[test]
            fn compress(ref input in any::<utils::InputStream>()) {
                let compressed = utils::deflate::stream::compress(input.stream());
                let output = utils::deflate::sync::decompress(&compressed);
                assert_eq!(output, input.bytes());
            }

            #[test]
            fn decompress(
                ref input in any::<Vec<u8>>(),
                chunk_size in 1..20usize,
            ) {
                let compressed = utils::deflate::sync::compress(input);
                let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
                let output = utils::deflate::stream::decompress(stream.stream());
                assert_eq!(&output, input);
            }
        }
    }

    mod bufread {
        use crate::utils;
        use proptest::{prelude::any, proptest};
        use std::iter::FromIterator;
        proptest! {
            #[test]
            fn compress(ref input in any::<utils::InputStream>()) {
                let compressed = utils::deflate::bufread::compress(input.reader());
                let output = utils::deflate::sync::decompress(&compressed);
                assert_eq!(output, input.bytes());
            }

            #[test]
            fn decompress(
                ref input in any::<Vec<u8>>(),
                chunk_size in 1..20usize,
            ) {
                let compressed = utils::deflate::sync::compress(input);
                let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
                let output = utils::deflate::bufread::decompress(stream.reader());
                assert_eq!(&output, input);
            }
        }
    }
}

mod zlib {
    mod stream {
        use crate::utils;
        use proptest::{prelude::any, proptest};
        use std::iter::FromIterator;
        proptest! {
            #[test]
            fn compress(ref input in any::<utils::InputStream>()) {
                let compressed = utils::zlib::stream::compress(input.stream());
                let output = utils::zlib::sync::decompress(&compressed);
                assert_eq!(output, input.bytes());
            }

            #[test]
            fn decompress(
                ref input in any::<Vec<u8>>(),
                chunk_size in 1..20usize,
            ) {
                let compressed = utils::zlib::sync::compress(input);
                let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
                let output = utils::zlib::stream::decompress(stream.stream());
                assert_eq!(&output, input);
            }
        }
    }

    mod bufread {
        use crate::utils;
        use proptest::{prelude::any, proptest};
        use std::iter::FromIterator;
        proptest! {
            #[test]
            fn compress(ref input in any::<utils::InputStream>()) {
                let compressed = utils::zlib::bufread::compress(input.reader());
                let output = utils::zlib::sync::decompress(&compressed);
                assert_eq!(output, input.bytes());
            }

            #[test]
            fn decompress(
                ref input in any::<Vec<u8>>(),
                chunk_size in 1..20usize,
            ) {
                let compressed = utils::zlib::sync::compress(input);
                let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
                let output = utils::zlib::bufread::decompress(stream.reader());
                assert_eq!(&output, input);
            }
        }
    }
}

mod gzip {
    mod stream {
        use crate::utils;
        use proptest::{prelude::any, proptest};
        use std::iter::FromIterator;
        proptest! {
            #[test]
            fn compress(ref input in any::<utils::InputStream>()) {
                let compressed = utils::gzip::stream::compress(input.stream());
                let output = utils::gzip::sync::decompress(&compressed);
                assert_eq!(output, input.bytes());
            }

            #[test]
            fn decompress(
                ref input in any::<Vec<u8>>(),
                chunk_size in 1..20usize,
            ) {
                let compressed = utils::gzip::sync::compress(input);
                let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
                let output = utils::gzip::stream::decompress(stream.stream());
                assert_eq!(&output, input);
            }
        }
    }
}

mod zstd {
    mod stream {
        use crate::utils;
        use proptest::{prelude::any, proptest};
        use std::iter::FromIterator;
        proptest! {
            #[test]
            fn compress(ref input in any::<utils::InputStream>()) {
                let compressed = utils::zstd::stream::compress(input.stream());
                let output = utils::zstd::sync::decompress(&compressed);
                assert_eq!(output, input.bytes());
            }

            #[test]
            fn decompress(
                ref input in any::<Vec<u8>>(),
                chunk_size in 1..20usize,
            ) {
                let compressed = utils::zstd::sync::compress(input);
                let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
                let output = utils::zstd::stream::decompress(stream.stream());
                assert_eq!(&output, input);
            }
        }
    }
}
