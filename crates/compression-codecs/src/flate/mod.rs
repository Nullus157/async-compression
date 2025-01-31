pub use flate2;

mod decoder;
mod encoder;

pub use self::{decoder::FlateDecoder, encoder::FlateEncoder};
