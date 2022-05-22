//! Types which operate over [`AsyncWrite`](tokio::io::AsyncWrite) streams, both encoders and
//! decoders for various formats.

#[macro_use]
mod macros;

algos!(tokio::write<W>);
