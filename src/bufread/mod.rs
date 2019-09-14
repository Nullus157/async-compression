//! Types which operate over [`AsyncBufRead`](futures::io::AsyncBufRead) streams, both encoders and
//! decoders for various formats.

mod generic;
#[macro_use]
mod macros;

pub(crate) use generic::{Decoder, Encoder};

#[cfg(feature = "deflate")]
mod deflate;
#[cfg(feature = "zlib")]
mod zlib;

#[cfg(feature = "deflate")]
pub use deflate::{DeflateDecoder, DeflateEncoder};
#[cfg(feature = "zlib")]
pub use zlib::{ZlibDecoder, ZlibEncoder};
