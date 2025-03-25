//! This module contains lz4-specific types for async-compression.

pub use lz4::liblz4::BlockSize;
use lz4::{
    liblz4::{BlockChecksum, FrameType, LZ4FFrameInfo, LZ4FPreferences},
    BlockMode, ContentChecksum,
};

/// lz4 compression parameters builder. This is a stable wrapper around lz4's own encoder
/// params type, to abstract over different versions of the lz4 library.
///
/// See the [lz4 documentation](https://github.com/lz4/lz4/blob/dev/doc/lz4frame_manual.html)
/// for more information on these parameters.
///
/// # Examples
///
/// ```
/// use async_compression::lz4;
///
/// let params = lz4::EncoderParams::default()
///     .block_size(lz4::BlockSize::Max1MB)
///     .content_checksum(true);
/// ```
#[derive(Clone, Debug, Default)]
pub struct EncoderParams {
    block_size: Option<BlockSize>,
    block_checksum: Option<BlockChecksum>,
    content_checksum: Option<ContentChecksum>,
}

impl EncoderParams {
    /// Sets input block size.
    pub fn block_size(mut self, block_size: BlockSize) -> Self {
        self.block_size = Some(block_size);
        self
    }

    /// Add a 32-bit checksum of frame's decompressed data.
    pub fn content_checksum(mut self, enable: bool) -> Self {
        self.content_checksum = Some(if enable {
            ContentChecksum::ChecksumEnabled
        } else {
            ContentChecksum::NoChecksum
        });
        self
    }

    /// Each block followed by a checksum of block's compressed data.
    pub fn block_checksum(mut self, enable: bool) -> Self {
        self.block_checksum = Some(if enable {
            BlockChecksum::BlockChecksumEnabled
        } else {
            BlockChecksum::NoBlockChecksum
        });
        self
    }

    pub(crate) fn as_lz4(&self) -> LZ4FPreferences {
        let block_size_id = self.block_size.clone().unwrap_or(BlockSize::Default);
        let content_checksum_flag = self
            .content_checksum
            .clone()
            .unwrap_or(ContentChecksum::NoChecksum);
        let block_checksum_flag = self
            .block_checksum
            .clone()
            .unwrap_or(BlockChecksum::NoBlockChecksum);

        LZ4FPreferences {
            frame_info: LZ4FFrameInfo {
                block_size_id,
                block_mode: BlockMode::Linked,
                content_checksum_flag,
                frame_type: FrameType::Frame,
                content_size: 0,
                dict_id: 0,
                block_checksum_flag,
            },
            compression_level: 0,
            auto_flush: 0,
            favor_dec_speed: 0,
            reserved: [0; 3],
        }
    }
}
