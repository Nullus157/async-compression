use brotli2::CompressParams;
use futures::io::AsyncBufRead;

decoder! {
    /// A brotli decoder, or decompressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "brotli")))]
    BrotliDecoder
}

encoder! {
    /// A brotli encoder, or compressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "brotli")))]
    BrotliEncoder
}

impl<R: AsyncBufRead> BrotliEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    ///
    /// The `level` argument here is typically 0-11.
    pub fn new(reader: R, level: u32) -> Self {
        let mut params = CompressParams::new();
        params.quality(level);
        Self::from_params(reader, &params)
    }

    /// Creates a new encoder with a custom [`CompressParams`].
    pub fn from_params(reader: R, params: &CompressParams) -> Self {
        Self {
            inner: crate::bufread::generic::Encoder::new(
                reader,
                crate::codec::BrotliEncoder::new(params),
            ),
        }
    }
}
