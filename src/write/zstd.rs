use futures_io::AsyncWrite;

decoder! {
    /// A zstd decoder, or uncompressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "zstd")))]
    ZstdDecoder
}

encoder! {
    /// A zstd encoder, or compressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "zstd")))]
    ZstdEncoder
}

impl<W: AsyncWrite> ZstdEncoder<W> {
    /// Creates a new encoder which will take in uncompressed data and write it compressed to the
    /// given stream.
    ///
    /// The `level` argument here can range from 1-21. A level of `0` will use zstd's default, which is `3`.
    pub fn new(writer: W, level: i32) -> Self {
        Self {
            inner: crate::write::Encoder::new(writer, crate::codec::ZstdEncoder::new(level)),
        }
    }
}
