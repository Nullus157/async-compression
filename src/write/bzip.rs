use bzip2::Compression;
use futures_io::AsyncWrite;

decoder! {
    /// A bzip2 decoder, or decompressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "bzip")))]
    BzDecoder
}

encoder! {
    /// A bzip2 encoder, or compressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "bzip")))]
    BzEncoder
}

impl<W: AsyncWrite> BzEncoder<W> {
    /// Creates a new encoder which will take in uncompressed data and write it compressed to the
    /// given stream.
    pub fn new(writer: W, level: Compression) -> BzEncoder<W> {
        BzEncoder {
            inner: crate::write::Encoder::new(writer, crate::codec::BzEncoder::new(level, 30)),
        }
    }
}
