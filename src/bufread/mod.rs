//! Types which operate over [`AsyncBufRead`](futures_io::AsyncBufRead) streams, both encoders and
//! decoders for various formats.

mod generic;
#[macro_use]
mod macros;

#[cfg(feature = "brotli")]
mod brotli;
#[cfg(feature = "deflate")]
mod deflate;
#[cfg(feature = "gzip")]
mod gzip;
#[cfg(feature = "zlib")]
mod zlib;
#[cfg(feature = "zstd")]
mod zstd;

pub(crate) use generic::{Decoder, Encoder};

#[cfg(feature = "brotli")]
pub use self::brotli::{BrotliDecoder, BrotliEncoder};
#[cfg(feature = "deflate")]
pub use self::deflate::{DeflateDecoder, DeflateEncoder};
#[cfg(feature = "gzip")]
pub use self::gzip::{GzipDecoder, GzipEncoder};
#[cfg(feature = "zlib")]
pub use self::zlib::{ZlibDecoder, ZlibEncoder};
#[cfg(feature = "zstd")]
pub use self::zstd::{ZstdDecoder, ZstdEncoder};
