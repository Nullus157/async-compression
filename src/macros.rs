macro_rules! algos {
    (@algo $algo:ident [$algo_s:expr] $decoder:ident $encoder:ident<$inner:ident> $({ $($constructor:tt)* })*) => {
        #[cfg(feature = $algo_s)]
        decoder! {
            #[doc = concat!("A ", $algo_s, " decoder, or decompressor")]
            #[cfg(feature = $algo_s)]
            $decoder
        }

        #[cfg(feature = $algo_s)]
        encoder! {
            #[doc = concat!("A ", $algo_s, " encoder, or compressor.")]
            #[cfg(feature = $algo_s)]
            $encoder<$inner> {
                pub fn new(inner: $inner) -> Self {
                    Self::with_quality(inner, crate::Level::Default)
                }
            } $({ $($constructor)* })*
        }
    };

    ($($mod:ident)::+<$inner:ident>) => {
        algos!(@algo brotli ["brotli"] BrotliDecoder BrotliEncoder<$inner> {
            pub fn with_quality(inner: $inner, level: crate::Level) -> Self {
                let params = brotli::enc::backward_references::BrotliEncoderParams::default();
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::BrotliEncoder::new(level.into_brotli(params)),
                    ),
                }
            }
        });

        algos!(@algo bzip2 ["bzip2"] BzDecoder BzEncoder<$inner> {
            pub fn with_quality(inner: $inner, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::BzEncoder::new(level.into_bzip2(), 0),
                    ),
                }
            }
        });

        algos!(@algo deflate ["deflate"] DeflateDecoder DeflateEncoder<$inner> {
            pub fn with_quality(inner: $inner, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::DeflateEncoder::new(level.into_flate2()),
                    ),
                }
            }
        });

        algos!(@algo gzip ["gzip"] GzipDecoder GzipEncoder<$inner> {
            pub fn with_quality(inner: $inner, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::GzipEncoder::new(level.into_flate2()),
                    ),
                }
            }
        });

        algos!(@algo zlib ["zlib"] ZlibDecoder ZlibEncoder<$inner> {
            pub fn with_quality(inner: $inner, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::ZlibEncoder::new(level.into_flate2()),
                    ),
                }
            }
        });

        algos!(@algo zstd ["zstd"] ZstdDecoder ZstdEncoder<$inner> {
            pub fn with_quality(inner: $inner, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::ZstdEncoder::new(level.into_zstd()),
                    ),
                }
            }

            /// Creates a new encoder, using the specified compression level and parameters, which
            /// will read uncompressed data from the given stream and emit a compressed stream.
            pub fn with_quality_and_params(inner: $inner, level: crate::Level, params: &[crate::zstd::CParameter]) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::ZstdEncoder::new_with_params(level.into_zstd(), params),
                    ),
                }
            }
        });

        algos!(@algo xz ["xz"] XzDecoder XzEncoder<$inner> {
            pub fn with_quality(inner: $inner, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::XzEncoder::new(level.into_xz2()),
                    ),
                }
            }
        });

        algos!(@algo lzma ["lzma"] LzmaDecoder LzmaEncoder<$inner> {
            pub fn with_quality(inner: $inner, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::LzmaEncoder::new(level.into_xz2()),
                    ),
                }
            }
        });
    }
}
