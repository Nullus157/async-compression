use bytes::Bytes;
use flate2::Compression;
use futures::stream::Stream;
use std::io::Result;

decoder! {
    /// A zlib decoder, or decompressor.
    ///
    /// This structure implements a [`Stream`] interface and will read compressed data from an
    /// underlying stream and emit a stream of uncompressed data.
    #[cfg_attr(docsrs, doc(cfg(feature = "zlib")))]
    ZlibDecoder
}

encoder! {
    /// A zlib encoder, or compressor.
    ///
    /// This structure implements a [`Stream`] interface and will read uncompressed data from an
    /// underlying stream and emit a stream of compressed data.
    #[cfg_attr(docsrs, doc(cfg(feature = "zlib")))]
    ZlibEncoder
}

impl<S: Stream<Item = Result<Bytes>>> ZlibEncoder<S> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    pub fn new(stream: S, level: Compression) -> Self {
        Self {
            inner: crate::stream::generic::Encoder::new(
                stream,
                crate::codec::ZlibEncoder::new(level),
            ),
        }
    }
}
