use bzip2::Compression;
use futures_io::AsyncBufRead;

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

impl<R: AsyncBufRead> BzEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    pub fn new(read: R, level: Compression) -> BzEncoder<R> {
        BzEncoder {
            inner: crate::bufread::Encoder::new(read, crate::codec::BzEncoder::new(level, 30)),
        }
    }
}
