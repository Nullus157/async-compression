use std::{
    io::Result,
    pin::Pin,
    task::{Context, Poll},
};

/// See
#[cfg_attr(feature = "futures-io-03", doc = " [`futures_io_03::AsyncWrite`]")]
#[cfg_attr(not(feature = "futures-io-03"), doc = " `futures_io_03::AsyncWrite`")]
/// or
#[cfg_attr(feature = "tokio", doc = " [`tokio::io::AsyncWrite`]")]
#[cfg_attr(not(feature = "tokio"), doc = " `tokio::io::AsyncWrite`")]
/// for documentation.
///
/// This is a local copy of the trait for compatibility purposes, it will be replaced once `std`
/// adds central async IO traits.
///
/// # SemVer compatibility note
///
/// Once `std` adds central async IO traits this may become a direct re-export of them; it is
/// undecided whether that will be a breaking release. So it is invalid to implement this trait on
/// a type that will also implement the `std` async IO traits in the future.
pub trait AsyncWrite {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>>;
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;
}
