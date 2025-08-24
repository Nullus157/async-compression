//! Types which operate over [`AsyncWrite`](tokio::io::AsyncWrite) streams, both encoders and
//! decoders for various formats.

#[macro_use]
mod macros;
mod generic;

use self::generic::{Decoder, Encoder};
use crate::{buf_write::AsyncBufWriteTokio as AsyncBufWrite, BufWriter};

algos!(tokio::write<W>);
