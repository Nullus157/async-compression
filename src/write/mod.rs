//! Types which operate over [`AsyncWrite`](futures_io::AsyncWrite) streams, both encoders and
//! decoders for various formats.

mod generic;
#[macro_use]
mod macros;

mod buf_write;
mod buf_writer;

#[cfg(feature = "brotli")]
mod brotli;
#[cfg(feature = "bzip")]
mod bzip2;
#[cfg(feature = "deflate")]
mod deflate;
#[cfg(feature = "gzip")]
mod gzip;
#[cfg(feature = "zlib")]
mod zlib;
#[cfg(feature = "zstd")]
mod zstd;

use self::{
    buf_write::AsyncBufWrite,
    buf_writer::BufWriter,
    generic::{Decoder, Encoder},
};

#[cfg(feature = "brotli")]
pub use self::brotli::{BrotliDecoder, BrotliEncoder};
#[cfg(feature = "bzip")]
pub use self::bzip2::{BzDecoder, BzEncoder};
#[cfg(feature = "deflate")]
pub use self::deflate::{DeflateDecoder, DeflateEncoder};
#[cfg(feature = "gzip")]
pub use self::gzip::{GzipDecoder, GzipEncoder};
#[cfg(feature = "zlib")]
pub use self::zlib::{ZlibDecoder, ZlibEncoder};
#[cfg(feature = "zstd")]
pub use self::zstd::{ZstdDecoder, ZstdEncoder};
