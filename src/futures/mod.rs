//! Implementations for IO traits exported by `futures`.

#[cfg(feature = "futures::bufread")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures::bufread")))]
pub mod bufread;

#[cfg(feature = "futures::write")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures::write")))]
pub mod write;
