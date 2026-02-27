mod decoder;
mod encoder;

pub use self::decoder::SnappyDecoder;
pub use self::encoder::SnappyEncoder;

use std::io;

const STREAM_FRAME: &'static [u8] = b"\xFF\x06\x00\x00sNaPpY";

#[derive(Debug, Copy, Clone)]
struct FrameHeader {
    chunk_type: ChunkType,
    data_frame_length: u64,
}

#[derive(Debug, Copy, Clone)]
enum ChunkType {
    Stream,
    Compressed,
    Uncompressed,
    Padding,
    ReservedUnskippable(u8),
    ReservedSkippable(u8),
}

impl From<u8> for ChunkType {
    fn from(value: u8) -> Self {
        match value {
            0xFF => Self::Stream,
            0x00 => Self::Compressed,
            0x01 => Self::Uncompressed,
            0xFE => Self::Padding,
            0x02..=0x7f => Self::ReservedUnskippable(value),
            0x80..=0xFD => Self::ReservedSkippable(value),
        }
    }
}

impl From<ChunkType> for u8 {
    fn from(value: ChunkType) -> Self {
        match value {
            ChunkType::Stream => 0xFF,
            ChunkType::Compressed => 0x00,
            ChunkType::Uncompressed => 0x01,
            ChunkType::Padding => 0xFE,
            ChunkType::ReservedUnskippable(chunk_type) => chunk_type,
            ChunkType::ReservedSkippable(chunk_type) => chunk_type,
        }
    }
}

impl FrameHeader {
    fn parse(input: &[u8]) -> io::Result<Self> {
        let (header_part, _): (&[u8; 4], _) = input.split_first_chunk().ok_or(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Need a least 4 bytes to parse the frame's header",
        ))?;

        let chunk_type = ChunkType::from(header_part[0]);
        // SAFETY: header_part is guaranteed to have at least 4 bytes due to split_first_chunk
        let length_part: &[u8; 3] = header_part[1..].first_chunk().unwrap();

        let length = read_u24_le(length_part) as u64;

        Ok(Self {
            chunk_type,
            data_frame_length: length,
        })
    }
}

impl From<FrameHeader> for [u8; 4] {
    fn from(value: FrameHeader) -> Self {
        let frame_length = value.data_frame_length as u32;

        let mut header = [0u8; 4];
        header[0] = u8::from(value.chunk_type);
        // We're writing a little endian u24 from an u32 by removing the latest significant byte
        header[1..4].copy_from_slice(&frame_length.to_le_bytes()[..3]);
        header
    }
}

pub fn read_u24_le(slice: &[u8; 3]) -> u32 {
    slice[0] as u32 | (slice[1] as u32) << 8 | (slice[2] as u32) << 16
}

fn crc32c_masked(input: &[u8]) -> u32 {
    let sum = crc32c::crc32c(input);
    mask_crc(sum)
}

fn mask_crc(crc: u32) -> u32 {
    (crc.wrapping_shr(15) | crc.wrapping_shl(17)).wrapping_add(0xA282EAD8)
}
