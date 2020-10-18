macro_rules! algos {
    ($(pub mod $name:ident($feat:literal, $encoder:ident, $decoder:ident) { pub mod sync { $($tt:tt)* } })*) => {
        $(
            #[cfg(feature = $feat)]
            pub mod $name {
                pub mod sync { $($tt)* }

                #[cfg(feature = "stream")]
                pub mod stream {
                    pub use async_compression::stream::{$decoder as Decoder, $encoder as Encoder};
                    pub use crate::utils::impls::stream::to_vec;

                    use crate::utils::{Level, pin_mut, Stream, Bytes, Result};

                    pub fn compress(input: impl Stream<Item = Result<Bytes>>) -> Vec<u8> {
                        pin_mut!(input);
                        to_vec(Encoder::with_quality(input, Level::Fastest))
                    }

                    pub fn decompress(input: impl Stream<Item = Result<Bytes>>) -> Vec<u8> {
                        pin_mut!(input);
                        to_vec(Decoder::new(input))
                    }
                }

                #[cfg(feature = "futures-io")]
                pub mod futures_io {
                    pub mod read {
                        pub use crate::utils::impls::futures_io::read::to_vec;
                    }

                    pub mod bufread {
                        pub use async_compression::futures::bufread::{
                            $decoder as Decoder, $encoder as Encoder,
                        };
                        pub use crate::utils::impls::futures_io::bufread::from;

                        use crate::utils::{Level, pin_mut};
                        use futures::io::AsyncBufRead;

                        pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
                            pin_mut!(input);
                            super::read::to_vec(Encoder::with_quality(input, Level::Fastest))
                        }

                        pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
                            pin_mut!(input);
                            super::read::to_vec(Decoder::new(input))
                        }
                    }

                    pub mod write {
                        pub use async_compression::futures::write::{
                            $decoder as Decoder, $encoder as Encoder,
                        };
                        pub use crate::utils::impls::futures_io::write::to_vec;

                        use crate::utils::Level;

                        pub fn compress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                            to_vec(
                                input,
                                |input| Box::pin(Encoder::with_quality(input, Level::Fastest)),
                                limit,
                            )
                        }

                        pub fn decompress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                            to_vec(input, |input| Box::pin(Decoder::new(input)), limit)
                        }
                    }
                }

                #[cfg(feature = "tokio-02")]
                pub mod tokio_02 {
                    pub mod read {
                        pub use crate::utils::impls::tokio_02::read::to_vec;
                    }

                    pub mod bufread {
                        pub use async_compression::tokio_02::bufread::{
                            $decoder as Decoder, $encoder as Encoder,
                        };
                        pub use crate::utils::impls::tokio_02::bufread::from;

                        use crate::utils::{Level, pin_mut};
                        use tokio_02::io::AsyncBufRead;

                        pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
                            pin_mut!(input);
                            super::read::to_vec(Encoder::with_quality(input, Level::Fastest))
                        }

                        pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
                            pin_mut!(input);
                            super::read::to_vec(Decoder::new(input))
                        }
                    }

                    pub mod write {
                        pub use async_compression::tokio_02::write::{
                            $decoder as Decoder, $encoder as Encoder,
                        };
                        pub use crate::utils::impls::tokio_02::write::to_vec;

                        use crate::utils::Level;

                        pub fn compress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                            to_vec(
                                input,
                                |input| Box::pin(Encoder::with_quality(input, Level::Fastest)),
                                limit,
                            )
                        }

                        pub fn decompress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                            to_vec(input, |input| Box::pin(Decoder::new(input)), limit)
                        }
                    }
                }

                #[cfg(feature = "tokio-03")]
                pub mod tokio_03 {
                    pub mod read {
                        pub use crate::utils::impls::tokio_03::read::to_vec;
                    }

                    pub mod bufread {
                        pub use async_compression::tokio_03::bufread::{
                            $decoder as Decoder, $encoder as Encoder,
                        };
                        pub use crate::utils::impls::tokio_03::bufread::from;

                        use crate::utils::{Level, pin_mut};
                        use tokio_03::io::AsyncBufRead;

                        pub fn compress(input: impl AsyncBufRead) -> Vec<u8> {
                            pin_mut!(input);
                            super::read::to_vec(Encoder::with_quality(input, Level::Fastest))
                        }

                        pub fn decompress(input: impl AsyncBufRead) -> Vec<u8> {
                            pin_mut!(input);
                            super::read::to_vec(Decoder::new(input))
                        }
                    }

                    pub mod write {
                        pub use async_compression::tokio_03::write::{
                            $decoder as Decoder, $encoder as Encoder,
                        };
                        pub use crate::utils::impls::tokio_03::write::to_vec;

                        use crate::utils::Level;

                        pub fn compress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                            to_vec(
                                input,
                                |input| Box::pin(Encoder::with_quality(input, Level::Fastest)),
                                limit,
                            )
                        }

                        pub fn decompress(input: &[Vec<u8>], limit: usize) -> Vec<u8> {
                            to_vec(input, |input| Box::pin(Decoder::new(input)), limit)
                        }
                    }
                }
            }
        )*
    }
}

algos! {
    pub mod brotli("brotli", BrotliEncoder, BrotliDecoder) {
        pub mod sync {
            pub use crate::utils::impls::sync::to_vec;

            pub fn compress(bytes: &[u8]) -> Vec<u8> {
                use brotli::{enc::backward_references::BrotliEncoderParams, CompressorReader};
                let mut params = BrotliEncoderParams::default();
                params.quality = 1;
                to_vec(CompressorReader::with_params(bytes, 0, &params))
            }

            pub fn decompress(bytes: &[u8]) -> Vec<u8> {
                use brotli::Decompressor;
                to_vec(Decompressor::new(bytes, 0))
            }
        }
    }

    pub mod bzip2("bzip2", BzEncoder, BzDecoder) {
        pub mod sync {
            pub use crate::utils::impls::sync::to_vec;

            pub fn compress(bytes: &[u8]) -> Vec<u8> {
                use bzip2::{bufread::BzEncoder, Compression};
                to_vec(BzEncoder::new(bytes, Compression::fast()))
            }

            pub fn decompress(bytes: &[u8]) -> Vec<u8> {
                use bzip2::bufread::BzDecoder;
                to_vec(BzDecoder::new(bytes))
            }
        }
    }

    pub mod deflate("deflate", DeflateEncoder, DeflateDecoder) {
        pub mod sync {
            pub use crate::utils::impls::sync::to_vec;

            pub fn compress(bytes: &[u8]) -> Vec<u8> {
                use flate2::{bufread::DeflateEncoder, Compression};
                to_vec(DeflateEncoder::new(bytes, Compression::fast()))
            }

            pub fn decompress(bytes: &[u8]) -> Vec<u8> {
                use flate2::bufread::DeflateDecoder;
                to_vec(DeflateDecoder::new(bytes))
            }
        }
    }

    pub mod zlib("zlib", ZlibEncoder, ZlibDecoder) {
        pub mod sync {
            pub use crate::utils::impls::sync::to_vec;

            pub fn compress(bytes: &[u8]) -> Vec<u8> {
                use flate2::{bufread::ZlibEncoder, Compression};
                to_vec(ZlibEncoder::new(bytes, Compression::fast()))
            }

            pub fn decompress(bytes: &[u8]) -> Vec<u8> {
                use flate2::bufread::ZlibDecoder;
                to_vec(ZlibDecoder::new(bytes))
            }
        }
    }

    pub mod gzip("gzip", GzipEncoder, GzipDecoder) {
        pub mod sync {
            pub use crate::utils::impls::sync::to_vec;

            pub fn compress(bytes: &[u8]) -> Vec<u8> {
                use flate2::{bufread::GzEncoder, Compression};
                to_vec(GzEncoder::new(bytes, Compression::fast()))
            }

            pub fn decompress(bytes: &[u8]) -> Vec<u8> {
                use flate2::bufread::GzDecoder;
                to_vec(GzDecoder::new(bytes))
            }
        }
    }

    pub mod zstd("zstd", ZstdEncoder, ZstdDecoder) {
        pub mod sync {
            pub use crate::utils::impls::sync::to_vec;

            pub fn compress(bytes: &[u8]) -> Vec<u8> {
                use libzstd::stream::read::Encoder;
                use libzstd::DEFAULT_COMPRESSION_LEVEL;
                to_vec(Encoder::new(bytes, DEFAULT_COMPRESSION_LEVEL).unwrap())
            }

            pub fn decompress(bytes: &[u8]) -> Vec<u8> {
                use libzstd::stream::read::Decoder;
                to_vec(Decoder::new(bytes).unwrap())
            }
        }
    }

    pub mod xz("xz", XzEncoder, XzDecoder) {
        pub mod sync {
            pub use crate::utils::impls::sync::to_vec;

            pub fn compress(bytes: &[u8]) -> Vec<u8> {
                use xz2::bufread::XzEncoder;

                to_vec(XzEncoder::new(bytes, 0))
            }

            pub fn decompress(bytes: &[u8]) -> Vec<u8> {
                use xz2::bufread::XzDecoder;

                to_vec(XzDecoder::new(bytes))
            }
        }
    }

    pub mod lzma("lzma", LzmaEncoder, LzmaDecoder) {
        pub mod sync {
            pub use crate::utils::impls::sync::to_vec;

            pub fn compress(bytes: &[u8]) -> Vec<u8> {
                use xz2::bufread::XzEncoder;
                use xz2::stream::{LzmaOptions, Stream};

                to_vec(XzEncoder::new_stream(
                    bytes,
                    Stream::new_lzma_encoder(&LzmaOptions::new_preset(0).unwrap()).unwrap(),
                ))
            }

            pub fn decompress(bytes: &[u8]) -> Vec<u8> {
                use xz2::bufread::XzDecoder;
                use xz2::stream::Stream;

                to_vec(XzDecoder::new_stream(
                    bytes,
                    Stream::new_lzma_decoder(u64::max_value()).unwrap(),
                ))
            }
        }
    }
}
