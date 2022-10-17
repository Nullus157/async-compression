//! Implementations for IO traits exported by [`tokio` v1.0](::tokio).

pub mod bufread;
pub mod write;

pub trait AsyncFlush {
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<bool>>;
}
