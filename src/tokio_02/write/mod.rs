//! Types which operate over [`AsyncWrite`](tokio_02::io::AsyncWrite) streams, both encoders and
//! decoders for various formats.

#[macro_use]
mod macros;

algos!(tokio_02::write<W>);
