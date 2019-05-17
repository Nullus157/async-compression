use flate2::Compression;
use futures_io::AsyncWrite;

decoder! {
    /// A deflate decoder, or uncompressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "deflate")))]
    DeflateDecoder
}

encoder! {
    /// A deflate encoder, or compressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "deflate")))]
    DeflateEncoder
}

impl<W: AsyncWrite> DeflateEncoder<W> {
    /// Creates a new encoder which will take in uncompressed data and write it compressed to the
    /// given stream.
    pub fn new(writer: W, level: Compression) -> Self {
        Self {
            inner: crate::write::Encoder::new(writer, crate::codec::DeflateEncoder::new(level)),
        }
    }
}
