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
            /// The `level` argument here is typically 0-11.
            pub fn new(reader: $inner, level: u32) -> Self {
                let mut params = brotli2::CompressParams::new();
                params.quality(level);
                Self::from_params(reader, &params)
            }
        } {
            pub fn from_params(inner: $inner, params: &brotli2::CompressParams) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::BrotliEncoder::new(params),
                    ),
                }
            }
        });

        algos!(@algo bzip2 ["bzip2"] BzDecoder BzEncoder<$inner> {
            pub fn new(inner: $inner, level: bzip2::Compression) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::BzEncoder::new(level, 30),
                    ),
                }
            }
        });

        algos!(@algo deflate ["deflate"] DeflateDecoder DeflateEncoder<$inner> {
            pub fn new(inner: $inner, level: flate2::Compression) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::DeflateEncoder::new(level),
                    ),
                }
            }
        });

        algos!(@algo gzip ["gzip"] GzipDecoder GzipEncoder<$inner> {
            pub fn new(inner: $inner, level: flate2::Compression) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::GzipEncoder::new(level),
                    ),
                }
            }
        });

        algos!(@algo zlib ["zlib"] ZlibDecoder ZlibEncoder<$inner> {
            pub fn new(inner: $inner, level: flate2::Compression) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::ZlibEncoder::new(level),
                    ),
                }
            }
        });

        algos!(@algo zstd ["zstd"] ZstdDecoder ZstdEncoder<$inner> {
            /// The `level` argument here can range from 1-21. A level of `0` will use zstd's default, which is `3`.
            pub fn new(inner: $inner, level: i32) -> Self {
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
