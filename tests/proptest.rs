use async_compression::Level;

use ::proptest::{
    arbitrary::any,
    prop_oneof,
    strategy::{Just, Strategy},
};

mod utils;

#[allow(dead_code)]
fn any_level() -> impl Strategy<Value = Level> {
    prop_oneof![
        Just(Level::Fastest),
        Just(Level::Best),
        Just(Level::Default),
        any::<u32>().prop_map(Level::Precise),
    ]
}

macro_rules! tests {
    ($($name:ident($feat:literal)),* $(,)?) => {
        $(
            #[cfg(feature = $feat)]
            mod $name {
                #[cfg(feature = "stream")]
                mod stream {
                    use crate::utils::{algos::$name::{stream, sync}, InputStream};
                    use proptest::{prelude::{any, ProptestConfig}, proptest};
                    use std::iter::FromIterator;

                    proptest! {
                        #[test]
                        fn compress(ref input in any::<InputStream>()) {
                            let compressed = stream::compress(input.stream());
                            let output = sync::decompress(&compressed);
                            assert_eq!(output, input.bytes());
                        }

                        #[test]
                        fn decompress(
                            ref input in any::<Vec<u8>>(),
                            chunk_size in 1..20usize,
                        ) {
                            let compressed = sync::compress(input);
                            let stream = InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
                            let output = stream::decompress(stream.stream());
                            assert_eq!(&output, input);
                        }
                    }

                    proptest! {
                        #![proptest_config(ProptestConfig::with_cases(32))]

                        #[test]
                        fn compress_with_level(
                            ref input in any::<InputStream>(),
                            level in crate::any_level(),
                        ) {
                            let encoder = stream::Encoder::with_quality(input.stream(), level);
                            let compressed = stream::to_vec(encoder);
                            let output = sync::decompress(&compressed);
                            assert_eq!(output, input.bytes());
                        }
                    }
                }

                #[cfg(feature = "futures-io")]
                mod futures {
                    mod bufread {
                        use crate::utils::{algos::$name::{futures_io::{bufread, read}, sync}, InputStream};
                        use proptest::{prelude::{any, ProptestConfig}, proptest};
                        use std::iter::FromIterator;

                        proptest! {
                            #[test]
                            fn compress(ref input in any::<InputStream>()) {
                                let reader = bufread::from(input);
                                let compressed = bufread::compress(reader);
                                let output = sync::decompress(&compressed);
                                assert_eq!(output, input.bytes());
                            }

                            #[test]
                            fn decompress(
                                ref bytes in any::<Vec<u8>>(),
                                chunk_size in 1..20usize,
                            ) {
                                let compressed = sync::compress(bytes);
                                let input = InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
                                let reader = bufread::from(&input);
                                let output = bufread::decompress(reader);
                                assert_eq!(&output, bytes);
                            }
                        }

                        proptest! {
                            #![proptest_config(ProptestConfig::with_cases(32))]

                            #[test]
                            fn compress_with_level(
                                ref input in any::<InputStream>(),
                                level in crate::any_level(),
                            ) {
                                let reader = bufread::from(input);
                                let encoder = bufread::Encoder::with_quality(reader, level);
                                let compressed = read::to_vec(encoder);
                                let output = sync::decompress(&compressed);
                                assert_eq!(output, input.bytes());
                            }
                        }
                    }

                    mod write {
                        use crate::utils::{algos::$name::{futures_io::write, sync}, InputStream};
                        use proptest::{prelude::{any, ProptestConfig}, proptest};

                        proptest! {
                            #[test]
                            fn compress(
                                ref input in any::<InputStream>(),
                                limit in 1..20usize,
                            ) {
                                let compressed = write::compress(input.as_ref(), limit);
                                let output = sync::decompress(&compressed);
                                assert_eq!(output, input.bytes());
                            }
                        }

                        proptest! {
                            #![proptest_config(ProptestConfig::with_cases(32))]

                            #[test]
                            fn compress_with_level(
                                ref input in any::<InputStream>(),
                                limit in 1..20usize,
                                level in crate::any_level(),
                            ) {
                                let compressed = write::to_vec(
                                    input.as_ref(),
                                    |input| Box::pin(write::Encoder::with_quality(input, level)),
                                    limit,
                                );
                                let output = sync::decompress(&compressed);
                                assert_eq!(output, input.bytes());
                            }
                        }
                    }
                }

                #[cfg(feature = "tokio-02")]
                mod tokio_02 {
                    mod bufread {
                        use crate::utils::{algos::$name::{tokio_02::{read, bufread}, sync}, InputStream};
                        use proptest::{prelude::{any, ProptestConfig}, proptest};
                        use std::iter::FromIterator;

                        proptest! {
                            #[test]
                            fn compress(ref input in any::<InputStream>()) {
                                let compressed = bufread::compress(bufread::from(input));
                                let output = sync::decompress(&compressed);
                                assert_eq!(output, input.bytes());
                            }

                            #[test]
                            fn decompress(
                                ref bytes in any::<Vec<u8>>(),
                                chunk_size in 1..20usize,
                            ) {
                                let compressed = sync::compress(bytes);
                                let input = InputStream::from(Vec::from_iter(compressed.chunks(chunk_size).map(Vec::from)));
                                let output = bufread::decompress(bufread::from(&input));
                                assert_eq!(&output, bytes);
                            }
                        }

                        proptest! {
                            #![proptest_config(ProptestConfig::with_cases(32))]

                            #[test]
                            fn compress_with_level(
                                ref input in any::<InputStream>(),
                                level in crate::any_level(),
                            ) {
                                let encoder = bufread::Encoder::with_quality(bufread::from(input), level);
                                let compressed = read::to_vec(encoder);
                                let output = sync::decompress(&compressed);
                                assert_eq!(output, input.bytes());
                            }
                        }
                    }

                    mod write {
                        use crate::utils::{algos::$name::{tokio_02::write, sync}, InputStream};
                        use proptest::{prelude::{any, ProptestConfig}, proptest};

                        proptest! {
                            #[test]
                            fn compress(
                                ref input in any::<InputStream>(),
                                limit in 1..20usize,
                            ) {
                                let compressed = write::compress(input.as_ref(), limit);
                                let output = sync::decompress(&compressed);
                                assert_eq!(output, input.bytes());
                            }
                        }

                        proptest! {
                            #![proptest_config(ProptestConfig::with_cases(32))]

                            #[test]
                            fn compress_with_level(
                                ref input in any::<InputStream>(),
                                limit in 1..20usize,
                                level in crate::any_level(),
                            ) {
                                let compressed = write::to_vec(
                                    input.as_ref(),
                                    |input| Box::pin(write::Encoder::with_quality(input, level)),
                                    limit,
                                );
                                let output = sync::decompress(&compressed);
                                assert_eq!(output, input.bytes());
                            }
                        }
                    }
                }
            }
        )*
    }
}

mod proptest {
    tests! {
        brotli("brotli"),
        bzip2("bzip2"),
        deflate("deflate"),
        gzip("gzip"),
        lzma("lzma"),
        xz("xz"),
        zlib("zlib"),
        zstd("zstd"),
    }
}
