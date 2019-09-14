use bytes::Bytes;
use futures::stream::Stream;
use std::io::Result;

decoder! {
    /// A zstd decoder, or decompressor.
    ///
    /// This structure implements a [`Stream`] interface and will read compressed data from an
    /// underlying stream and emit a stream of uncompressed data.
    #[cfg_attr(docsrs, doc(cfg(feature = "zstd")))]
    ZstdDecoder
}

encoder! {
    /// A zstd encoder, or compressor.
    ///
    /// This structure implements a [`Stream`] interface and will read uncompressed data from an
    /// underlying stream and emit a stream of compressed data.
    #[cfg_attr(docsrs, doc(cfg(feature = "zstd")))]
    ZstdEncoder
}

impl<S: Stream<Item = Result<Bytes>>> ZstdEncoder<S> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    ///
    /// The `level` argument here can range from 1-21. A level of `0` will use zstd's default, which is `3`.
    pub fn new(stream: S, level: i32) -> Self {
        Self {
            inner: crate::stream::generic::Encoder::new(
                stream,
                crate::codec::ZstdEncoder::new(level),
            ),
        }
    }

}
