use flate2::Compression;
use futures::io::AsyncBufRead;

decoder! {
    /// A deflate decoder, or decompressor.
    ///
    /// This structure implements an [`AsyncRead`] interface and will read compressed data from an
    /// underlying stream and emit a stream of uncompressed data.
    #[cfg_attr(docsrs, doc(cfg(feature = "deflate")))]
    DeflateDecoder
}

encoder! {
    /// A deflate encoder, or compressor.
    ///
    /// This structure implements an [`AsyncRead`] interface and will read uncompressed data from an
    /// underlying stream and emit a stream of compressed data.
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
