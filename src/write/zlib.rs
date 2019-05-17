use flate2::Compression;
use futures_io::AsyncWrite;

decoder! {
    /// A zlib decoder, or uncompressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "zlib")))]
    ZlibDecoder
}

encoder! {
    /// A zlib encoder, or compressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "zlib")))]
    ZlibEncoder
}

impl<W: AsyncWrite> ZlibEncoder<W> {
    /// Creates a new encoder which will take in uncompressed data and write it compressed to the
    /// given stream.
    pub fn new(writer: W, level: Compression) -> Self {
        Self {
            inner: crate::write::Encoder::new(writer, crate::codec::ZlibEncoder::new(level)),
        }
    }
}
