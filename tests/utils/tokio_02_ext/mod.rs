#[cfg(feature = "tokio-02-bufread")]
mod copy_buf;
#[cfg(feature = "tokio-02-write")]
mod interleave_pending;
#[cfg(feature = "tokio-02-write")]
mod limited;

#[cfg(feature = "tokio-02-bufread")]
pub use copy_buf::copy_buf;

#[cfg(feature = "tokio-02-write")]
pub trait AsyncWriteTestExt: tokio_02::io::AsyncWrite {
    fn interleave_pending_write(self) -> interleave_pending::InterleavePending<Self>
    where
        Self: Sized + Unpin,
    {
        interleave_pending::InterleavePending::new(self)
    }

    fn limited_write(self, limit: usize) -> limited::Limited<Self>
    where
        Self: Sized + Unpin,
    {
        limited::Limited::new(self, limit)
    }
}

#[cfg(feature = "tokio-02-write")]
impl<T: tokio_02::io::AsyncWrite> AsyncWriteTestExt for T {}
