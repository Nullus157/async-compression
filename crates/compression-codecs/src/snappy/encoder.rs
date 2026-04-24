use crate::snappy::{crc32c_masked, ChunkType, FrameHeader, MAX_FRAME_SIZE, STREAM_FRAME};
use crate::EncodeV2;
use compression_core::util::{PartialBuffer, WriteBuffer};

const MAX_BLOCK_SIZE: usize = 1 << 16;

#[derive(Debug)]
pub struct SnappyEncoder {
    state: State,
    in_buf: PartialBuffer<Vec<u8>>,
    out_buf: PartialBuffer<Vec<u8>>,
}

impl Default for SnappyEncoder {
    fn default() -> Self {
        Self {
            state: State::InitStream(PartialBuffer::new(STREAM_FRAME)),
            in_buf: PartialBuffer::new(Vec::with_capacity(MAX_BLOCK_SIZE)),
            out_buf: PartialBuffer::new(Vec::with_capacity(MAX_FRAME_SIZE)),
        }
    }
}

impl SnappyEncoder {
    pub fn new() -> Self {
        Self::default()
    }

    fn compress_frame(&mut self) -> std::io::Result<()> {
        let in_buffer = &self.in_buf.unwritten();
        let checksum = crc32c_masked(in_buffer);

        self.out_buf.reset();
        let out_buf = self.out_buf.get_mut();
        out_buf.clear();
        let max_compress_size = snap::raw::max_compress_len(in_buffer.len());
        out_buf.resize(max_compress_size + 8, 0);

        let mut encoder = snap::raw::Encoder::new();
        let compress_data = encoder.compress(in_buffer, &mut out_buf[8..])?;

        let (chunk_type, chunk_len) = if compress_data >= in_buffer.len() - (in_buffer.len() / 8) {
            (ChunkType::Uncompressed, in_buffer.len())
        } else {
            out_buf.truncate(compress_data);
            (ChunkType::Compressed, out_buf.len())
        };

        // We add 4 because the length includes the 4 bytes of the checksum.
        let chunk_len = chunk_len + 4;
        let header = FrameHeader {
            chunk_type,
            data_frame_length: chunk_len as u64,
        };

        let mut raw_chunk_header = [0u8; 8];
        let raw_frame_header: [u8; 4] = header.into();
        let raw_checksum: [u8; 4] = checksum.to_le_bytes();

        raw_chunk_header[0..4].copy_from_slice(&raw_frame_header);
        raw_chunk_header[4..8].copy_from_slice(&raw_checksum);

        match chunk_type {
            ChunkType::Compressed => self.state = State::CompressCopy(raw_chunk_header.into()),
            ChunkType::Uncompressed => self.state = State::UncompressCopy(raw_chunk_header.into()),
            _ => unreachable!(),
        }

        Ok(())
    }
}

fn write(
    header: &mut PartialBuffer<[u8; 8]>,
    input: &mut PartialBuffer<Vec<u8>>,
    output: &mut WriteBuffer<'_>,
) -> bool {
    if !header.unwritten().is_empty() {
        output.copy_unwritten_from(header);
        if output.has_no_spare_space() {
            return false;
        }
    }

    if !input.unwritten().is_empty() {
        output.copy_unwritten_from(input);
        false
    } else {
        true
    }
}

#[derive(Debug)]
enum State {
    InitStream(PartialBuffer<&'static [u8]>),
    Buffering,
    UncompressCopy(PartialBuffer<[u8; 8]>),
    CompressCopy(PartialBuffer<[u8; 8]>),
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
                    if !buffer.unwritten().is_empty() {
                        output.copy_unwritten_from(buffer);
                        if output.has_no_spare_space() {
                            return Ok(());
                        }
                    }
                    self.state = State::Buffering
                }
                State::Buffering => {
                    let buffer = self.in_buf.get_mut();
                    let input_buf = input.unwritten();
                    let available = MAX_BLOCK_SIZE - buffer.len();
                    let boundary = available.min(input_buf.len());
                    let input_buf = &input_buf[..boundary];

                    buffer.extend_from_slice(input_buf);
                    input.advance(input_buf.len());

                    if buffer.len() < MAX_BLOCK_SIZE {
                        return Ok(());
                    }

                    self.compress_frame()?;
                }
                State::UncompressCopy(header) => {
                    if !write(header, &mut self.in_buf, output) {
                        return Ok(());
                    }
                    self.in_buf.get_mut().clear();
                    self.in_buf.reset();
                    self.state = State::Buffering
                }
                State::CompressCopy(header) => {
                    if !write(header, &mut self.out_buf, output) {
                        return Ok(());
                    }
                    self.in_buf.get_mut().clear();
                    self.in_buf.reset();
                    self.state = State::Buffering
                }
            }
        }
    }

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> std::io::Result<bool> {
        loop {
            match &mut self.state {
                State::InitStream(buffer) => {
                    if !buffer.unwritten().is_empty() {
                        output.copy_unwritten_from(buffer);
                        if output.has_no_spare_space() {
                            return Ok(false);
                        }
                    }
                    self.state = State::Buffering
                }
                State::Buffering => {
                    self.compress_frame()?;
                }
                State::UncompressCopy(header) => {
                    return Ok(write(header, &mut self.in_buf, output))
                }
                State::CompressCopy(header) => return Ok(write(header, &mut self.out_buf, output)),
            }
        }
    }

    fn finish(&mut self, output: &mut WriteBuffer<'_>) -> std::io::Result<bool> {
        self.flush(output)
    }
}
