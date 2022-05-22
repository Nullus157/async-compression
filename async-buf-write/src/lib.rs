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
