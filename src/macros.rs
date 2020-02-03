macro_rules! algos {
    (@algo $algo:ident [$algo_s:expr] $decoder:ident $encoder:ident<$inner:ident> $({ $($constructor:tt)* })*) => {
        #[cfg(feature = $algo_s)]
        decoder! {
            /// A
            #[doc = $algo_s]
            /// decoder, or decompressor.
            #[cfg_attr(docsrs, doc(cfg(feature = $algo_s)))]
            $decoder
        }

        #[cfg(feature = $algo_s)]
        encoder! {
            /// A
            #[doc = $algo_s]
            /// encoder, or compressor.
            #[cfg_attr(docsrs, doc(cfg(feature = $algo_s)))]
            $encoder<$inner> $({ $($constructor)* })*
        }
    };

    ($($mod:ident)::+<$inner:ident>) => {
        algos!(@algo brotli ["brotli"] BrotliDecoder BrotliEncoder<$inner> {
            pub fn new(reader: $inner) -> Self {
                let params = brotli::enc::backward_references::BrotliEncoderParams::default();
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        reader,
                        crate::codec::BrotliEncoder::new(params),
                    ),
                }
            }
        } {
            /// The `level` argument here is typically 0-11.
            pub fn with_quality(inner: $inner, level: crate::Compression) -> Self {
                let mut params = brotli::enc::backward_references::BrotliEncoderParams::default();
                match level {
                    crate::Compression::Fastest => params.quality = 0,
                    crate::Compression::Best => params.quality = 11,
                    crate::Compression::Default => (),
                }
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::BrotliEncoder::new(params),
                    ),
                }
            }
        });

        algos!(@algo bzip2 ["bzip2"] BzDecoder BzEncoder<$inner> {
            pub fn new(inner: $inner) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::BzEncoder::new(crate::Compression::Default.into(), 0),
                    ),
                }
            }
        } {
            pub fn with_quality(inner: $inner, level: crate::Compression) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::BzEncoder::new(level.into(), 0),
                    ),
                }
            }
        });

        algos!(@algo deflate ["deflate"] DeflateDecoder DeflateEncoder<$inner> {
            pub fn new(inner: $inner) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::DeflateEncoder::new(crate::Compression::Default.into()),
                    ),
                }
            }
        } {
            pub fn with_quality(inner: $inner, level: crate::Compression) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::DeflateEncoder::new(level.into()),
                    ),
                }
            }
        });

        algos!(@algo gzip ["gzip"] GzipDecoder GzipEncoder<$inner> {
            pub fn new(inner: $inner) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::GzipEncoder::new(crate::Compression::Default.into()),
                    ),
                }
            }
        } {
            pub fn with_quality(inner: $inner, level: crate::Compression) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::GzipEncoder::new(level.into()),
                    ),
                }
            }
        });

        algos!(@algo zlib ["zlib"] ZlibDecoder ZlibEncoder<$inner> {
            pub fn new(inner: $inner) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::ZlibEncoder::new(crate::Compression::Default.into()),
                    ),
                }
            }
        } {
            pub fn with_quality(inner: $inner, level: crate::Compression) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::ZlibEncoder::new(level.into()),
                    ),
                }
            }
        });

        algos!(@algo zstd ["zstd"] ZstdDecoder ZstdEncoder<$inner> {
            /// The `level` argument here can range from 1-21. A level of `0` will use zstd's default, which is `3`.
            pub fn new(inner: $inner) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::ZstdEncoder::new(0),
                    ),
                }
            }
        } {
            pub fn with_quality(inner: $inner, level: crate::Compression) -> Self {
                let level = match level {
                    crate::Compression::Fastest => 1,
                    crate::Compression::Best => 21,
                    crate::Compression::Default => 0,
                };
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::ZstdEncoder::new(level),
                    ),
                }
            }
        });
    }
}
