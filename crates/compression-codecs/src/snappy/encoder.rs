use crate::snappy::{crc32c_masked, ChunkType, FrameHeader, STREAM_FRAME};
use crate::EncodeV2;
use compression_core::util::{PartialBuffer, WriteBuffer};

const MAX_BLOCK_SIZE: usize = 1 << 16;

#[derive(Debug)]
pub struct SnappyEncoder {
    state: State,
    chunk: Vec<u8>,
}

impl Default for SnappyEncoder {
    fn default() -> Self {
        Self {
            state: State::InitStream(PartialBuffer::new(STREAM_FRAME)),
            chunk: Vec::with_capacity(MAX_BLOCK_SIZE),
        }
    }
}

impl SnappyEncoder {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug)]
enum State {
    InitStream(PartialBuffer<&'static [u8]>),
    Buffering,
    Writing(PartialBuffer<Vec<u8>>),
}

fn compress_frame(buffer: &[u8]) -> std::io::Result<Vec<u8>> {
    let checksum = crc32c_masked(&buffer);

    let mut encoder = snap::raw::Encoder::new();
    let compress_data = encoder.compress_vec(&buffer)?;
    let (chunk_type, data) = if compress_data.len() >= buffer.len() - (buffer.len() / 8) {
        (ChunkType::Uncompressed, buffer)
    } else {
        (ChunkType::Compressed, compress_data.as_slice())
    };

    // We add 4 because the length includes the 4 bytes of the checksum.
    let chunk_len = data.len() + 4;
    let header = FrameHeader {
        chunk_type,
        data_frame_length: chunk_len as u64,
    };

    let mut frame = Vec::with_capacity(data.len() + 8);
    let raw_header: [u8; 4] = header.into();
    let raw_checksum: [u8; 4] = checksum.to_le_bytes();

    frame.extend_from_slice(&raw_header);
    frame.extend_from_slice(&raw_checksum);
    frame.extend_from_slice(&data);

    Ok(frame)
}

impl EncodeV2 for SnappyEncoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> std::io::Result<()> {
        loop {
            match &mut self.state {
                State::InitStream(buffer) => {
                    if buffer.unwritten().len() > 0 {
                        output.copy_unwritten_from(buffer);
                        if output.has_no_spare_space() {
                            return Ok(());
                        }
                    }
                    self.state = State::Buffering
                }
                State::Buffering => {
                    let buffer = &mut self.chunk;
                    let input_buf = input.unwritten();
                    let available = MAX_BLOCK_SIZE - buffer.len();
                    let boundary = available.min(input_buf.len());
                    let input_buf = &input_buf[..boundary];

                    buffer.extend_from_slice(input_buf);
                    input.advance(input_buf.len());

                    if buffer.len() < MAX_BLOCK_SIZE {
                        return Ok(());
                    }

                    let compressed_frame = compress_frame(buffer)?;
                    buffer.clear();
                    self.state = State::Writing(compressed_frame.into())
                }
                State::Writing(buffer) => {
                    if buffer.unwritten().len() > 0 {
                        output.copy_unwritten_from(buffer);
                        return Ok(());
                    } else {
                        self.state = State::Buffering
                    }
                }
            }
        }
    }

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> std::io::Result<bool> {
        loop {
            match &mut self.state {
                State::InitStream(buffer) => {
                    if buffer.unwritten().len() > 0 {
                        output.copy_unwritten_from(buffer);
                        if output.has_no_spare_space() {
                            return Ok(false);
                        }
                    }
                    self.state = State::Buffering
                }
                State::Buffering => {
                    let buffer = &mut self.chunk;
                    let compressed_data = compress_frame(&buffer)?;
                    buffer.clear();
                    self.state = State::Writing(compressed_data.into())
                }
                State::Writing(buffer) => {
                    return if buffer.unwritten().len() > 0 {
                        output.copy_unwritten_from(buffer);
                        Ok(false)
                    } else {
                        Ok(true)
                    }
                }
            }
        }
    }

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> std::io::Result<bool> {
        self.flush(output)
    }
}
