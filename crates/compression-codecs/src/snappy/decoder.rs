use crate::snappy::{mask_crc, ChunkType, FrameHeader};
use crate::DecodeV2;
use compression_core::util::{PartialBuffer, WriteBuffer};
use std::convert::TryInto;
use std::{io, mem};

#[derive(Debug, Default)]
pub struct SnappyDecoder {
    state: State,
}

impl SnappyDecoder {
    pub fn new() -> Self {
        Self::default()
    }
}

fn decode_chunk(chunk_type: ChunkType, mut buffer: Vec<u8>) -> std::io::Result<Vec<u8>> {
    let data = buffer.split_off(4);

    let expected_sum: [u8; 4] = buffer
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid checksum length"))?;
    let expected_sum = u32::from_le_bytes(expected_sum);

    let output = match chunk_type {
        ChunkType::Compressed => {
            let uncompress_length = snap::raw::decompress_len(&data)?;
            let mut out_buf = vec![0; uncompress_length];
            let mut decoder = snap::raw::Decoder::new();
            decoder.decompress(&data, &mut out_buf)?;
            out_buf
        }
        ChunkType::Uncompressed => data,
        _ => unreachable!(
            "can only decode compressed or uncompressed chunks, not {:?}",
            chunk_type
        ),
    };

    let got_sum = crc32c::crc32c(&output);
    let got_sum = mask_crc(got_sum);
    if expected_sum != got_sum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "checksum mismatch",
        ));
    }

    Ok(output)
}

#[derive(Debug)]
enum State {
    StreamIdentifier(PartialBuffer<[u8; 4]>),
    ChunkHeader(PartialBuffer<[u8; 4]>),
    Skipping(usize),
    Buffering {
        remaining: usize,
        chunk_type: ChunkType,
        buffer: Vec<u8>,
    },
    Sending(PartialBuffer<Vec<u8>>),
}

impl Default for State {
    fn default() -> Self {
        State::StreamIdentifier(PartialBuffer::new([0; 4]))
    }
}

impl DecodeV2 for SnappyDecoder {
    fn reinit(&mut self) -> std::io::Result<()> {
        *self = Self::new();
        Ok(())
    }

    fn decode(
        &mut self,
        input: &mut PartialBuffer<&[u8]>,
        output: &mut WriteBuffer<'_>,
    ) -> std::io::Result<bool> {
        loop {
            match &mut self.state {
                State::StreamIdentifier(header) => {
                    header.copy_unwritten_from(input);
                    if !header.unwritten().is_empty() {
                        return Ok(false);
                    }

                    let header = FrameHeader::parse(header.written())?;
                    if let ChunkType::Stream = header.chunk_type {
                        self.state = State::Skipping(header.data_frame_length as usize)
                    } else {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "Invalid chunk type, expected Stream, got: {:?}",
                                header.chunk_type
                            ),
                        ));
                    }
                }
                State::ChunkHeader(header) => {
                    header.copy_unwritten_from(input);
                    if !header.unwritten().is_empty() {
                        return Ok(false);
                    }

                    let header = FrameHeader::parse(header.written())?;

                    let data_frame_length = header.data_frame_length as usize;

                    match header.chunk_type {
                        ChunkType::Stream
                        | ChunkType::ReservedSkippable(_)
                        | ChunkType::Padding => self.state = State::Skipping(data_frame_length),
                        ChunkType::Compressed | ChunkType::Uncompressed => {
                            self.state = State::Buffering {
                                remaining: data_frame_length,
                                chunk_type: header.chunk_type,
                                buffer: Vec::with_capacity(data_frame_length),
                            }
                        }
                        ChunkType::ReservedUnskippable(chunk_type) => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!(
                                    "Reserved unskippable chunk type encountered: {}",
                                    chunk_type
                                ),
                            ))
                        }
                    }
                }
                State::Skipping(n) => {
                    let input_len = input.unwritten().len();
                    if input_len < *n {
                        input.advance(input_len);
                        *n -= input_len;
                        return Ok(false);
                    }
                    input.advance(*n);
                    self.state = State::ChunkHeader([0u8; 4].into())
                }
                State::Buffering {
                    remaining,
                    chunk_type,
                    buffer,
                } => {
                    let input_buf = input.unwritten();
                    let boundary = (*remaining).min(input_buf.len());
                    let input_buf = &input_buf[..boundary];

                    *remaining -= input_buf.len();

                    buffer.extend_from_slice(input_buf);
                    input.advance(input_buf.len());

                    if *remaining != 0 {
                        return Ok(false);
                    }

                    // We're done buffering, so let's decode the chunk
                    let chunk_type = *chunk_type;
                    let buffer = mem::take(buffer);
                    let output = decode_chunk(chunk_type, buffer)?;
                    self.state = State::Sending(PartialBuffer::new(output))
                }
                State::Sending(buffer) => {
                    output.copy_unwritten_from(buffer);
                    if buffer.unwritten().is_empty() {
                        self.state = State::ChunkHeader([0u8; 4].into())
                    } else {
                        return Ok(false);
                    }
                }
            }
        }
    }

    fn flush(&mut self, output: &mut WriteBuffer<'_>) -> std::io::Result<bool> {
        match &mut self.state {
            State::Sending(buffer) => {
                output.copy_unwritten_from(buffer);
                if buffer.unwritten().is_empty() {
                    self.state = State::ChunkHeader([0u8; 4].into());
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(true),
        }
    }

    fn finish(&mut self, _output: &mut WriteBuffer<'_>) -> std::io::Result<bool> {
        match &mut self.state {
            State::ChunkHeader(header) if header.unwritten().len() == 4 => Ok(true),
            _ => Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
        }
    }
}
