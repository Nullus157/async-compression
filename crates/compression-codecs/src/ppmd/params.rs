use compression_core::Level;

#[derive(Debug, Clone, Copy)]
pub struct PpmdEncoderParams {
    pub order: u32,
    pub memory_size: u32,
    pub end_marker: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PpmdDecoderParams {
    pub order: u32,
    pub memory_size: u32,
}

impl From<Level> for PpmdEncoderParams {
    fn from(_value: Level) -> Self {
        todo!()
    }
}
