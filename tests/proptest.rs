mod utils;

macro_rules! tests {
    ($($name:ident),*) => {
        $(
            mod $name {
                mod stream {
                    use crate::utils;
                    use proptest::{prelude::any, proptest};
                    use std::iter::FromIterator;
                    proptest! {
                        #[test]
                        fn compress(ref input in any::<utils::InputStream>()) {
                            let compressed = utils::$name::stream::compress(input.stream());
                            let output = utils::$name::sync::decompress(&compressed);
                            assert_eq!(output, input.bytes());
                        }

                        #[test]
                        fn decompress(
                            ref input in any::<Vec<u8>>(),
                            chunk_size in 1..20usize,
                        ) {
                            let compressed = utils::$name::sync::compress(input);
                            let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
                            let output = utils::$name::stream::decompress(stream.stream());
                            assert_eq!(&output, input);
                        }
                    }
                }

                mod futures {
                    mod bufread {
                        use crate::utils;
                        use proptest::{prelude::any, proptest};
                        use std::iter::FromIterator;
                        proptest! {
                            #[test]
                            fn compress(ref input in any::<utils::InputStream>()) {
                                let compressed = utils::$name::futures::bufread::compress(input.reader());
                                let output = utils::$name::sync::decompress(&compressed);
                                assert_eq!(output, input.bytes());
                            }

                            #[test]
                            fn decompress(
                                ref input in any::<Vec<u8>>(),
                                chunk_size in 1..20usize,
                            ) {
                                let compressed = utils::$name::sync::compress(input);
                                let stream = utils::InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
                                let output = utils::$name::futures::bufread::decompress(stream.reader());
                                assert_eq!(&output, input);
                            }
                        }
                    }

                    mod write {
                        use crate::utils;
                        use proptest::{prelude::any, proptest};
                        proptest! {
                            #[test]
                            fn compress(
                                ref input in any::<utils::InputStream>(),
                                limit in 1..20usize,
                            ) {
                                let compressed = utils::$name::futures::write::compress(input.as_ref(), limit);
                                let output = utils::$name::sync::decompress(&compressed);
                                assert_eq!(output, input.bytes());
                            }
                        }
                    }
                }
            }
        )*
    }
}

tests!(brotli, bzip2, deflate, gzip, zlib, zstd, xz);
