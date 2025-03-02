use std::io::{self, Result};

use lz4::liblz4::{
    check_error, BlockChecksum, BlockMode, BlockSize, ContentChecksum, FrameType,
    LZ4FCompressionContext, LZ4FFrameInfo, LZ4FPreferences, LZ4F_compressBegin, LZ4F_compressBound,
    LZ4F_compressEnd, LZ4F_compressUpdate, LZ4F_createCompressionContext, LZ4F_flush,
    LZ4F_freeCompressionContext, LZ4F_VERSION,
};

use crate::{codec::Encode, unshared::Unshared, util::PartialBuffer};

#[derive(Debug)]
struct EncoderContext {
    ctx: LZ4FCompressionContext,
}

#[derive(Debug)]
enum State {
    Init {
        preferences: LZ4FPreferences,
        buffer: PartialBuffer<Vec<u8>>,
    },
    Encoding {
        buffer: PartialBuffer<Vec<u8>>,
    },
    Footer {
        buffer: PartialBuffer<Vec<u8>>,
    },
    Done,
}

#[derive(Debug)]
pub struct Lz4Encoder {
    ctx: Unshared<EncoderContext>,
    state: State,
    limit: usize,
}

impl EncoderContext {
    fn new() -> Result<Self> {
        let mut context = LZ4FCompressionContext(core::ptr::null_mut());
        check_error(unsafe { LZ4F_createCompressionContext(&mut context, LZ4F_VERSION) })?;
        Ok(Self { ctx: context })
    }
}

impl Drop for EncoderContext {
    fn drop(&mut self) {
        unsafe { LZ4F_freeCompressionContext(self.ctx) };
    }
}

impl Lz4Encoder {
    pub fn new(level: u32) -> Self {
        let block_size = BlockSize::Default.get_size();
        let preferences = LZ4FPreferences {
            frame_info: LZ4FFrameInfo {
                block_size_id: BlockSize::Default,
                block_mode: BlockMode::Linked,
                content_checksum_flag: ContentChecksum::ChecksumEnabled,
                content_size: 0,
                frame_type: FrameType::Frame,
                dict_id: 0,
                block_checksum_flag: BlockChecksum::BlockChecksumEnabled,
            },
            compression_level: level,
            auto_flush: 0,
            favor_dec_speed: 0,
            reserved: [0; 3],
        };

        let buffer_size = unsafe { LZ4F_compressBound(block_size, &preferences) };

        Self {
            ctx: Unshared::new(EncoderContext::new().unwrap()),
            state: State::Init {
                preferences,
                buffer: PartialBuffer::new(Vec::with_capacity(buffer_size)),
            },
            limit: block_size,
        }
    }

    fn init(
        ctx: LZ4FCompressionContext,
        preferences: &LZ4FPreferences,
        buffer: &mut PartialBuffer<Vec<u8>>,
    ) -> Result<()> {
        unsafe {
            let len = check_error(LZ4F_compressBegin(
                ctx,
                buffer.unwritten_mut().as_mut_ptr(),
                buffer.get_mut().capacity(),
                preferences,
            ))?;
            buffer.get_mut().set_len(len);
        };
        Ok(())
    }
}

impl Encode for Lz4Encoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<()> {
        loop {
            match &mut self.state {
                State::Init {
                    preferences,
                    buffer,
                } => {
                    Self::init(self.ctx.get_mut().ctx, preferences, buffer);

                    self.state = State::Encoding {
                        buffer: buffer.take(),
                    }
                }

                State::Encoding { buffer } => {
                    output.copy_unwritten_from(buffer);

                    if buffer.unwritten().is_empty() {
                        buffer.reset();
                        let size = input.unwritten().len().min(self.limit);
                        unsafe {
                            let len = check_error(LZ4F_compressUpdate(
                                self.ctx.get_mut().ctx,
                                buffer.unwritten_mut().as_mut_ptr(),
                                buffer.get_mut().capacity(),
                                input.unwritten().as_ptr(),
                                size,
                                core::ptr::null(),
                            ))?;
                            buffer.get_mut().set_len(len);
                        }
                        input.advance(size);
                    }
                }

                State::Footer { .. } | State::Done => {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "encode after complete",
                    ));
                }
            }

            if input.unwritten().is_empty() || output.unwritten().is_empty() {
                return Ok(());
            }
        }
    }

    fn flush(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        loop {
            let done = match &mut self.state {
                State::Init {
                    preferences,
                    buffer,
                } => {
                    Self::init(self.ctx.get_mut().ctx, preferences, buffer);
                    self.state = State::Encoding {
                        buffer: buffer.take(),
                    };
                    false
                }

                State::Encoding { buffer } => {
                    output.copy_unwritten_from(buffer);

                    if buffer.unwritten().is_empty() {
                        buffer.reset();
                        unsafe {
                            let len = check_error(LZ4F_flush(
                                self.ctx.get_mut().ctx,
                                buffer.unwritten_mut().as_mut_ptr(),
                                buffer.get_mut().capacity(),
                                core::ptr::null(),
                            ))?;
                            buffer.get_mut().set_len(len);
                            len == 0
                        }
                    } else {
                        false
                    }
                }

                State::Footer { buffer } => {
                    output.copy_unwritten_from(buffer);

                    if buffer.unwritten().is_empty() {
                        self.state = State::Done;
                        true
                    } else {
                        false
                    }
                }

                State::Done => true,
            };

            if done {
                return Ok(true);
            }

            if output.unwritten().is_empty() {
                return Ok(false);
            }
        }
    }

    fn finish(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<bool> {
        loop {
            match &mut self.state {
                State::Init {
                    preferences,
                    buffer,
                } => {
                    Self::init(self.ctx.get_mut().ctx, preferences, buffer);

                    self.state = State::Encoding {
                        buffer: buffer.take(),
                    }
                }

                State::Encoding { buffer } => {
                    output.copy_unwritten_from(buffer);

                    if buffer.unwritten().is_empty() {
                        buffer.reset();
                        unsafe {
                            let len = check_error(LZ4F_compressEnd(
                                self.ctx.get_mut().ctx,
                                buffer.unwritten_mut().as_mut_ptr(),
                                buffer.get_mut().capacity(),
                                core::ptr::null(),
                            ))?;
                            buffer.get_mut().set_len(len);
                        }
                        self.state = State::Footer {
                            buffer: buffer.take(),
                        };
                    }
                }

                State::Footer { buffer } => {
                    output.copy_unwritten_from(buffer);

                    if buffer.unwritten().is_empty() {
                        self.state = State::Done;
                    }
                }

                State::Done => {}
            }

            if let State::Done = self.state {
                return Ok(true);
            }

            if output.unwritten().is_empty() {
                return Ok(false);
            }
        }
    }
}
