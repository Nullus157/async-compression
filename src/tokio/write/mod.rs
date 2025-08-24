//! Types which operate over [`AsyncWrite`](tokio::io::AsyncWrite) streams, both encoders and
//! decoders for various formats.

#[macro_use]
mod macros;
mod generic;

use crate::{buf_write::AsyncBufWriteTokio as AsyncBufWrite, BufWriter};
use self::generic::{Decoder, Encoder};

algos!(tokio::write<W>);
