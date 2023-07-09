macro_rules! algos {
    (@algo $algo:ident [$algo_s:expr] $decoder:ident<$inner_dec:ident> $encoder:ident<$inner_enc:ident> $({ $($constructor:tt)* })*) => {
        algos!(@algo-dec $algo [$algo_s] $decoder<$inner_dec>);
        algos!(@algo-enc $algo [$algo_s] $encoder<$inner_enc> $({ $($constructor)* })*);
    };

    (@algo-dec $algo:ident [$algo_s:expr] $decoder:ident<$inner:ident> $({ $($constructor:tt)* })*) => {
        #[cfg(feature = $algo_s)]
        decoder! {
            #[doc = concat!("A ", $algo_s, " decoder, or decompressor")]
            #[cfg(feature = $algo_s)]
            $decoder<$inner>
            $({ $($constructor)* })*
        }
    };

    (@algo-enc $algo:ident [$algo_s:expr] $encoder:ident<$inner:ident> $({ $($constructor:tt)* })*) => {
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

    ($($mod:ident)::+<$inner_enc:ident, $inner_dec:ident>) => {
        algos!(@algo brotli ["brotli"] BrotliDecoder<$inner_dec> BrotliEncoder<$inner_enc> {
            pub fn with_quality(inner: $inner_enc, level: crate::Level) -> Self {
                let params = brotli::enc::backward_references::BrotliEncoderParams::default();
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::BrotliEncoder::new(level.into_brotli(params)),
                    ),
                }
            }
        });

        algos!(@algo bzip2 ["bzip2"] BzDecoder<$inner_dec> BzEncoder<$inner_enc> {
            pub fn with_quality(inner: $inner_enc, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::BzEncoder::new(level.into_bzip2(), 0),
                    ),
                }
            }
        });

        algos!(@algo deflate ["deflate"] DeflateDecoder<$inner_dec> DeflateEncoder<$inner_enc> {
            pub fn with_quality(inner: $inner_enc, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::DeflateEncoder::new(level.into_flate2()),
                    ),
                }
            }
        });

        algos!(@algo gzip ["gzip"] GzipDecoder<$inner_dec> GzipEncoder<$inner_enc> {
            pub fn with_quality(inner: $inner_enc, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::GzipEncoder::new(level.into_flate2()),
                    ),
                }
            }
        });

        algos!(@algo zlib ["zlib"] ZlibDecoder<$inner_dec> ZlibEncoder<$inner_enc> {
            pub fn with_quality(inner: $inner_enc, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::ZlibEncoder::new(level.into_flate2()),
                    ),
                }
            }
        });


        algos!(@algo-enc zstd ["zstd"] ZstdEncoder<$inner_enc> {
            pub fn with_quality(inner: $inner_enc, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::ZstdEncoder::new(level.into_zstd()),
                    ),
                }
            }

            /// Creates a new encoder, using the specified compression level and pre-trained
            /// dictionary, which will read uncompressed data from the given stream and emit
            /// a compressed stream.
            ///
            /// (Dictionaries provide better compression ratios for small files,
            /// but are required to be present during decompression.)
            pub fn with_dict(inner: $inner_enc, level: crate::Level, dictionary: &[u8]) -> ::std::io::Result<Self> {
                Ok(Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::ZstdEncoder::new_with_dict(level.into_zstd(), dictionary)?,
                    ),
                })
            }

            /// Creates a new encoder, using the specified compression level and parameters, which
            /// will read uncompressed data from the given stream and emit a compressed stream.
            ///
            /// # Panics
            ///
            /// Panics if this function is called with a [`CParameter::nb_workers()`] parameter and
            /// the `zstdmt` crate feature is _not_ enabled.
            ///
            /// [`CParameter::nb_workers()`]: crate::zstd::CParameter
            //
            // TODO: remove panic note on next breaking release, along with `CParameter::nb_workers`
            // change
            pub fn with_quality_and_params(inner: $inner, level: crate::Level, params: &[crate::zstd::CParameter]) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::ZstdEncoder::new_with_params(level.into_zstd(), params),
                    ),
                }
            }
        });

        algos!(@algo-dec zstd ["zstd"] ZstdDecoder<$inner_dec> {
            /// Creates a new decoder, using a pre-trained dictionary, which will read
            /// compressed data from the given stream and emit an uncompressed stream.
            ///
            /// (Dictionaries provide better compression ratios for small files but
            /// you must use the same dictionary for both encoding and decoding data)
            pub fn with_dict(read: $inner_dec, dictionary: &[u8]) -> ::std::io::Result<Self> {
                Ok(Self {
                    inner: crate::$($mod::)+generic::Decoder::new(
                        read,
                        crate::codec::ZstdDecoder::new_with_dict(dictionary)?,
                    ),
                })
            }
        });

        algos!(@algo xz ["xz"] XzDecoder<$inner_dec> XzEncoder<$inner_enc> {
            pub fn with_quality(inner: $inner_enc, level: crate::Level) -> Self {
                Self {
                    inner: crate::$($mod::)+generic::Encoder::new(
                        inner,
                        crate::codec::XzEncoder::new(level.into_xz2()),
                    ),
                }
            }
        });

        algos!(@algo lzma ["lzma"] LzmaDecoder<$inner_dec> LzmaEncoder<$inner_enc> {
            pub fn with_quality(inner: $inner_enc, level: crate::Level) -> Self {
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
