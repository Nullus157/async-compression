use flate2::Compression;
use futures::io::AsyncBufRead;

decoder! {
    /// A zlib decoder, or decompressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "zlib")))]
    ZlibDecoder
}

encoder! {
    /// A zlib encoder, or compressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "zlib")))]
    ZlibEncoder
}

impl<R: AsyncBufRead> ZlibEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    pub fn new(read: R, level: Compression) -> ZlibEncoder<R> {
        ZlibEncoder {
            inner: crate::bufread::Encoder::new(read, crate::codec::ZlibEncoder::new(level)),
        }
    }
}
