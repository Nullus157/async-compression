use brotli2::CompressParams;
use futures_io::AsyncWrite;

decoder! {
    /// A brotli decoder, or uncompressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "brotli")))]
    BrotliDecoder
}

encoder! {
    /// A brotli encoder, or compressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "brotli")))]
    BrotliEncoder
}

impl<W: AsyncWrite> BrotliEncoder<W> {
    /// Creates a new encoder which will take in uncompressed data and write it compressed to the
    /// given stream.
    ///
    /// The `level` argument here is typically 0-11.
    pub fn new(writer: W, level: u32) -> Self {
        let mut params = CompressParams::new();
        params.quality(level);
        Self::from_params(writer, &params)
    }

    /// Creates a new encoder with a custom [`CompressParams`].
    pub fn from_params(writer: W, params: &CompressParams) -> Self {
        Self {
            inner: crate::write::Encoder::new(writer, crate::codec::BrotliEncoder::new(params)),
        }
    }
}
