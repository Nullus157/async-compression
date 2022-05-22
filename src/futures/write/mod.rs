//! Types which operate over [`AsyncWrite`](futures_io::AsyncWrite) streams, both encoders and
//! decoders for various formats.

#[macro_use]
mod macros;

algos!(futures::write<W>);
