use flate2::Compression;
use futures_io::AsyncBufRead;

decoder! {
    /// A deflate decoder, or decompressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "deflate")))]
    DeflateDecoder
}

encoder! {
    /// A deflate encoder, or compressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "deflate")))]
    DeflateEncoder
}

impl<R: AsyncBufRead> DeflateEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    pub fn new(read: R, level: Compression) -> DeflateEncoder<R> {
        DeflateEncoder {
            inner: crate::bufread::Encoder::new(read, crate::codec::DeflateEncoder::new(level)),
        }
    }
}
