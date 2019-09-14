use brotli2::CompressParams;
use bytes::Bytes;
use futures::stream::Stream;
use std::io::Result;

decoder! {
    /// A brotli decoder, or decompressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "brotli")))]
    BrotliDecoder
}

encoder! {
    /// A brotli encoder, or compressor.
    #[cfg_attr(docsrs, doc(cfg(feature = "brotli")))]
    BrotliEncoder
}

impl<S: Stream<Item = Result<Bytes>>> BrotliEncoder<S> {
    /// Creates a new encoder which will read uncompressed data from the given stream and emit a
    /// compressed stream.
    ///
    /// The `level` argument here is typically 0-11.
    pub fn new(stream: S, level: u32) -> BrotliEncoder<S> {
        let mut params = CompressParams::new();
        params.quality(level);
        BrotliEncoder::from_params(stream, &params)
    }

    /// Creates a new encoder with a custom [`CompressParams`].
    pub fn from_params(stream: S, params: &CompressParams) -> BrotliEncoder<S> {
        BrotliEncoder {
            inner: crate::stream::generic::Encoder::new(
                stream,
                crate::codec::BrotliEncoder::new(params),
            ),
        }
    }
}
