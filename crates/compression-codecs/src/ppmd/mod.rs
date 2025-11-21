use compression_core::Level;

mod decoder;
mod encoder;
pub mod params;

pub use decoder::PpmdDecoder;
pub use encoder::PpmdEncoder;

impl PpmdEncoder {
    pub fn new(level: Level) -> Self {
        Self::from_params(params::PpmdEncoderParams::from(level))
    }
}
