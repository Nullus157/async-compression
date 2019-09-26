use bytes::Bytes;
use flate2::Compression;
use futures_core::stream::Stream;
use std::io::Result;

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

impl<S: Stream<Item = Result<Bytes>>> DeflateEncoder<S> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    pub fn new(stream: S, level: Compression) -> Self {
        Self {
            inner: crate::stream::generic::Encoder::new(
                stream,
                crate::codec::DeflateEncoder::new(level),
            ),
        }
    }
}
