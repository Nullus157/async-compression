use futures::io::AsyncBufRead;

decoder! {
    /// A zstd decoder, or decompressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "zstd")))]
    ZstdDecoder
}

encoder! {
    /// A zstd encoder, or compressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "zstd")))]
    ZstdEncoder
}

impl<R: AsyncBufRead> ZstdEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    ///
    /// The `level` argument here can range from 1-21. A level of `0` will use zstd's default, which is `3`.
    pub fn new(reader: R, level: i32) -> Self {
        Self {
            inner: crate::bufread::generic::Encoder::new(
                reader,
                crate::codec::ZstdEncoder::new(level),
            ),
        }
    }
}
