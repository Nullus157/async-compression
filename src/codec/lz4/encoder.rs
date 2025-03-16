use std::io::{self, Result};

use lz4::liblz4::{
    check_error, BlockChecksum, BlockMode, BlockSize, ContentChecksum, FrameType,
    LZ4FCompressionContext, LZ4FFrameInfo, LZ4FPreferences, LZ4F_compressBegin, LZ4F_compressBound,
    LZ4F_compressEnd, LZ4F_compressUpdate, LZ4F_createCompressionContext, LZ4F_flush,
    LZ4F_freeCompressionContext, LZ4F_VERSION,
};

use crate::{codec::Encode, lz4::EncoderParams, unshared::Unshared, util::PartialBuffer};

// https://github.com/lz4/lz4/blob/9d53d8bb6c4120345a0966e5d8b16d7def1f32c5/lib/lz4frame.h#L281
const LZ4F_HEADER_SIZE_MAX: usize = 19;

#[derive(Debug)]
struct EncoderContext {
    ctx: LZ4FCompressionContext,
}

#[derive(Clone, Copy, Debug)]
enum State {
    Header,
    Encoding,
    Footer,
    Done,
}

enum Lz4Fn<'a, T>
where
    T: AsRef<[u8]>,
{
    Begin,
    Update { input: &'a mut PartialBuffer<T> },
    Flush,
    End,
}

#[derive(Debug)]
pub struct Lz4Encoder {
    ctx: Unshared<EncoderContext>,
    state: State,
    preferences: LZ4FPreferences,
    limit: usize,
    maybe_buffer: Option<PartialBuffer<Vec<u8>>>,
    /// Minimum dst buffer size for a block
    block_buffer_size: usize,
    /// Minimum dst buffer size for flush/end
    flush_buffer_size: usize,
}

// minimum size of destination buffer for compressing `src_size` bytes
fn min_dst_size(src_size: usize, preferences: &LZ4FPreferences) -> usize {
    unsafe { LZ4F_compressBound(src_size, preferences) }
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
    pub(crate) fn new(preferences: LZ4FPreferences) -> Self {
        let block_size = preferences.frame_info.block_size_id.get_size();

        let block_buffer_size = min_dst_size(block_size, &preferences);
        let flush_buffer_size = min_dst_size(0, &preferences);

        Self {
            ctx: Unshared::new(EncoderContext::new().unwrap()),
            state: State::Header,
            preferences,
            limit: block_size,
            maybe_buffer: None,
            block_buffer_size,
            flush_buffer_size,
        }
    }

    pub(crate) fn buffer_size(&self) -> usize {
        self.block_buffer_size
    }

    fn write<'a, T>(
        &'a mut self,
        lz4_fn: Lz4Fn<'a, T>,
        output: &'a mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<usize>
    where
        T: AsRef<[u8]>,
    {
        let min_dst_size = match &lz4_fn {
            Lz4Fn::Begin => LZ4F_HEADER_SIZE_MAX,
            Lz4Fn::Update { input } => {
                min_dst_size(input.unwritten().len().min(self.limit), &self.preferences)
            }
            Lz4Fn::Flush | Lz4Fn::End => self.flush_buffer_size,
        };

        let direct_output = output.unwritten().len() >= min_dst_size;

        let (dst_buffer, dst_max_size) = if direct_output {
            let output_len = output.unwritten().len();
            (output.unwritten_mut(), output_len)
        } else {
            let buffer_size = self.block_buffer_size;
            let buffer = self
                .maybe_buffer
                .get_or_insert_with(|| PartialBuffer::new(Vec::with_capacity(buffer_size)));
            buffer.reset();
            (buffer.unwritten_mut(), buffer_size)
        };

        let len = match lz4_fn {
            Lz4Fn::Begin => {
                let len = check_error(unsafe {
                    LZ4F_compressBegin(
                        self.ctx.get_mut().ctx,
                        dst_buffer.as_mut_ptr(),
                        dst_max_size,
                        &self.preferences,
                    )
                })?;
                self.state = State::Encoding;
                len
            }
            Lz4Fn::Update { input } => {
                let size = input.unwritten().len().min(self.limit);
                let len = check_error(unsafe {
                    LZ4F_compressUpdate(
                        self.ctx.get_mut().ctx,
                        dst_buffer.as_mut_ptr(),
                        dst_max_size,
                        input.unwritten().as_ptr(),
                        size,
                        core::ptr::null(),
                    )
                })?;
                input.advance(size);
                len
            }
            Lz4Fn::Flush => check_error(unsafe {
                LZ4F_flush(
                    self.ctx.get_mut().ctx,
                    dst_buffer.as_mut_ptr(),
                    dst_max_size,
                    core::ptr::null(),
                )
            })?,
            Lz4Fn::End => {
                let len = check_error(unsafe {
                    LZ4F_compressEnd(
                        self.ctx.get_mut().ctx,
                        dst_buffer.as_mut_ptr(),
                        dst_max_size,
                        core::ptr::null(),
                    )
                })?;
                self.state = State::Footer;
                len
            }
        };

        if direct_output {
            output.advance(len);
        } else {
            // SAFETY: buffer is initialized above incase of a non-direct operation
            unsafe {
                self.maybe_buffer
                    .as_mut()
                    .unwrap_unchecked()
                    .get_mut()
                    .set_len(len);
            }
        }

        Ok(len)
    }
}

impl Encode for Lz4Encoder {
    fn encode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> Result<()> {
        loop {
            match self.state {
                State::Header => {
                    self.write(Lz4Fn::Begin::<&[u8]>, output)?;
                }

                State::Encoding => {
                    if let Some(buffer) = self.maybe_buffer.as_mut() {
                        output.copy_unwritten_from(buffer);
                    }

                    // start another round of compression if buffer is fully drained or None
                    if self
                        .maybe_buffer
                        .as_ref()
                        .is_none_or(|buffer| buffer.unwritten().is_empty())
                    {
                        self.write(Lz4Fn::Update { input }, output)?;
                    }
                }

                State::Footer | State::Done => {
                    return Err(io::Error::other("encode after complete"));
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
            let done = match self.state {
                State::Header => {
                    self.write(Lz4Fn::Begin::<&[u8]>, output)?;
                    false
                }

                State::Encoding => {
                    if let Some(buffer) = self.maybe_buffer.as_mut() {
                        output.copy_unwritten_from(buffer);
                    }

                    if self
                        .maybe_buffer
                        .as_ref()
                        .is_none_or(|buffer| buffer.unwritten().is_empty())
                    {
                        let len = self.write(Lz4Fn::Flush::<&[u8]>, output)?;
                        len == 0
                    } else {
                        false
                    }
                }

                State::Footer => {
                    if let Some(buffer) = self.maybe_buffer.as_mut() {
                        output.copy_unwritten_from(buffer);
                    }

                    if self
                        .maybe_buffer
                        .as_ref()
                        .is_none_or(|buffer| buffer.unwritten().is_empty())
                    {
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
            match self.state {
                State::Header => {
                    self.write(Lz4Fn::Begin::<&[u8]>, output)?;
                }

                State::Encoding => {
                    if let Some(buffer) = self.maybe_buffer.as_mut() {
                        output.copy_unwritten_from(buffer);
                    }

                    if self
                        .maybe_buffer
                        .as_ref()
                        .is_none_or(|buffer| buffer.unwritten().is_empty())
                    {
                        self.write(Lz4Fn::End::<&[u8]>, output)?;
                    }
                }

                State::Footer => {
                    if let Some(buffer) = self.maybe_buffer.as_mut() {
                        output.copy_unwritten_from(buffer);
                    }

                    if self
                        .maybe_buffer
                        .as_ref()
                        .is_none_or(|buffer| buffer.unwritten().is_empty())
                    {
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
