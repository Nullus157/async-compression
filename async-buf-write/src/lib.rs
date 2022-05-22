/// Sometimes when writing data to an IO sink you need to write it into a buffered slice before
/// writing it out to the underlying device. Managing this buffer can be annoying, so this crate
/// deals with that for you (and exposes a trait that could allow IO sinks that can support this
/// themself to deal with it more efficiently).
#[macro_use]
mod compat;

mod async_buf_write;
mod async_buf_writer;
mod async_write;

#[cfg(feature = "futures-io-03")]
pub mod futures_io_03;

#[cfg(feature = "tokio")]
pub mod tokio;

#[cfg(feature = "tokio-02")]
pub mod tokio_02;

#[cfg(feature = "tokio-03")]
pub mod tokio_03;

pub use self::{
    async_buf_write::AsyncBufWrite, async_buf_writer::AsyncBufWriter, async_write::AsyncWrite,
};
