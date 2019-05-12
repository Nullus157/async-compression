//! Types which operate over [`AsyncBufRead`](futures::io::AsyncBufRead) streams, both encoders and
//! decoders for various formats.

mod deflate;
mod zlib;

pub use deflate::DeflateEncoder;
pub use zlib::ZlibEncoder;
