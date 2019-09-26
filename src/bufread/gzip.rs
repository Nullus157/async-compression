use flate2::Compression;
use futures_io::AsyncBufRead;

decoder! {
    /// A gzip decoder, or decompressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "gzip")))]
    GzipDecoder
}

encoder! {
    /// A gzip encoder, or compressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "gzip")))]
    GzipEncoder
}

impl<R: AsyncBufRead> GzipEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    pub fn new(read: R, level: Compression) -> GzipEncoder<R> {
        GzipEncoder {
            inner: crate::bufread::Encoder::new(read, crate::codec::GzipEncoder::new(level)),
        }
    }
}
