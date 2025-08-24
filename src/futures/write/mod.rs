//! Types which operate over [`AsyncWrite`](futures_io::AsyncWrite) streams, both encoders and
//! decoders for various formats.

#[macro_use]
mod macros;
mod generic;

use crate::{buf_write::AsyncBufWriteFuturesIo as AsyncBufWrite, BufWriter};
use self::generic::{Decoder, Encoder};

algos!(futures::write<W>);
