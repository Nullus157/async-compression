//! Types which operate over [`AsyncWrite`](futures_io::AsyncWrite) streams, both encoders and
//! decoders for various formats.

#[macro_use]
mod macros;
mod generic;

mod buf_writer;

use crate::AsyncBufWrite;
use self::{
    buf_writer::BufWriter,
    generic::{Decoder, Encoder},
};

algos!(futures::write<W>);
