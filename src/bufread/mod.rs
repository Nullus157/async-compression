//! Types which operate over [`AsyncBufRead`](futures::io::AsyncBufRead) streams, both encoders and
//! decoders for various formats.

mod generic;

pub(crate) use generic::Encoder;

#[cfg(feature = "deflate")]
mod deflate;
#[cfg(feature = "zlib")]
mod zlib;

#[cfg(feature = "deflate")]
pub use deflate::DeflateEncoder;
#[cfg(feature = "zlib")]
pub use zlib::ZlibEncoder;
