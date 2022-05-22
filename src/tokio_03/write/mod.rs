//! Types which operate over [`AsyncWrite`](tokio_03::io::AsyncWrite) streams, both encoders and
//! decoders for various formats.

#[macro_use]
mod macros;

algos!(tokio_03::write<W>);
