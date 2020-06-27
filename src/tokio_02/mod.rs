//! Implementations for IO traits exported by [`tokio` v0.2](::tokio_02).

#[cfg(feature = "tokio-02-bufread")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-02-bufread")))]
pub mod bufread;

#[cfg(feature = "tokio-02-write")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-02-write")))]
pub mod write;
