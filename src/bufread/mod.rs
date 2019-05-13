//! Types which operate over [`AsyncBufRead`](futures::io::AsyncBufRead) streams, both encoders and
//! decoders for various formats.

#[cfg(feature = "deflate")]
mod deflate;
#[cfg(feature = "zlib")]
mod zlib;

#[cfg(feature = "deflate")]
pub use deflate::DeflateEncoder;
#[cfg(feature = "zlib")]
pub use zlib::ZlibEncoder;
