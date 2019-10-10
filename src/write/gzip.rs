use flate2::Compression;
use futures_io::AsyncWrite;

decoder! {
    /// A gzip decoder, or uncompressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "gzip")))]
    GzipDecoder
}

encoder! {
    /// A gzip encoder, or compressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "gzip")))]
    GzipEncoder
}

impl<W: AsyncWrite> GzipEncoder<W> {
    /// Creates a new encoder which will take in uncompressed data and write it compressed to the
    /// given stream.
    pub fn new(writer: W, level: Compression) -> Self {
        Self {
            inner: crate::write::Encoder::new(writer, crate::codec::GzipEncoder::new(level)),
        }
    }
}
