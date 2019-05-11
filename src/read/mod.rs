//! Types which operate over [`AsyncBufRead`](futures::io::AsyncBufRead) streams, both encoders and
//! decoders for various formats.

//pub mod brotli;
pub mod deflate;
pub mod zlib;
