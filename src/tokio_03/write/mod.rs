//! Types which operate over [`AsyncWrite`](tokio_03::io::AsyncWrite) streams, both encoders and
//! decoders for various formats.

#[macro_use]
mod macros;
mod generic;

use self::generic::{Decoder, Encoder};

algos!(tokio_03::write<W>);
