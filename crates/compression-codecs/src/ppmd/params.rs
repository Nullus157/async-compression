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
    fn from(value: Level) -> Self {
        use ppmd_rust::{PPMD7_MAX_MEM_SIZE, PPMD7_MAX_ORDER, PPMD7_MIN_MEM_SIZE, PPMD7_MIN_ORDER};

        let (order, mem) = match value {
            Level::Fastest => (4u32, 1 << 20),
            Level::Best => (16u32, 16 << 20),
            Level::Precise(q) => {
                let order = (q as u32).clamp(PPMD7_MIN_ORDER, PPMD7_MAX_ORDER);
                let steps = (order.saturating_sub(PPMD7_MIN_ORDER)) / 4;
                let mem = (4 << 20) + steps.saturating_mul(4 << 20);
                (order, mem)
            }
            _ => (8u32, 4 << 20),
        };

        let memory_size = mem.clamp(PPMD7_MIN_MEM_SIZE, PPMD7_MAX_MEM_SIZE);
        Self {
            order,
            memory_size,
            end_marker: true,
        }
    }
}
