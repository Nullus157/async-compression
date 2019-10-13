use bytes::Bytes;
use bzip2::Compression;
use futures_core::stream::Stream;
use std::io::Result;

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

impl<S: Stream<Item = Result<Bytes>>> BzEncoder<S> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    pub fn new(stream: S, level: Compression) -> Self {
        Self {
            inner: crate::stream::generic::Encoder::new(
                stream,
                crate::codec::BzEncoder::new(level, 30),
            ),
        }
    }
}
