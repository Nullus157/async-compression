/// Buffer containing partially written data.
#[derive(Debug, Default)]
pub struct PartialBuffer<B: AsRef<[u8]>> {
    /// Underlying buffer.
    buffer: B,
    /// Index up to which data has been written.
    index: usize,
}

impl<B: AsRef<[u8]>> PartialBuffer<B> {
    /// Create a new [`PartialBuffer`] from the given underlying buffer.
    pub fn new(buffer: B) -> Self {
        Self { buffer, index: 0 }
    }

    /// Written part of the buffer.
    pub fn written(&self) -> &[u8] {
        &self.buffer.as_ref()[..self.index]
    }

    /// Unwritten part of the buffer.
    pub fn unwritten(&self) -> &[u8] {
        &self.buffer.as_ref()[self.index..]
    }

    /// Advance the written part.
    pub fn advance(&mut self, amount: usize) {
        self.index += amount;
    }

    /// Mutable reference to the underlying buffer.
    pub fn get_mut(&mut self) -> &mut B {
        &mut self.buffer
    }

    /// Convert the [`PartialBuffer`] back into the underlying buffer.
    pub fn into_inner(self) -> B {
        self.buffer
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> PartialBuffer<B> {
    /// Mutable reference to the unwritten part of the buffer.
    pub fn unwritten_mut(&mut self) -> &mut [u8] {
        &mut self.buffer.as_mut()[self.index..]
    }

    /// Copy the unwritten part of another buffer into this buffer, advancing both buffers.
    pub fn copy_unwritten_from<C: AsRef<[u8]>>(&mut self, other: &mut PartialBuffer<C>) {
        let len = std::cmp::min(self.unwritten().len(), other.unwritten().len());

        self.unwritten_mut()[..len].copy_from_slice(&other.unwritten()[..len]);

        self.advance(len);
        other.advance(len);
    }
}

impl<B: AsRef<[u8]> + Default> PartialBuffer<B> {
    /// Take out the underlying buffer replacing it with a default.
    pub fn take(&mut self) -> Self {
        core::mem::replace(self, Self::new(B::default()))
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> From<B> for PartialBuffer<B> {
    fn from(buffer: B) -> Self {
        Self::new(buffer)
    }
}

/// Abstraction for encoders.
pub trait Encode {
    /// Encode the provided input buffer into the provided output buffer.
    fn encode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> std::io::Result<()>;

    /// Flush the internal buffers into the provided output buffer.
    ///
    /// Returns `true` iff the internal buffers have been completely flushed.
    fn flush(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> std::io::Result<bool>;

    /// Finish the encoding into the provided output buffer.
    ///
    /// Returns `true` iff the internal buffers have been completely flushed.
    fn finish(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> std::io::Result<bool>;
}

/// Abstraction for decoders.
pub trait Decode {
    /// Reinitializes this decoder, preparing it to decode a new member/frame of data.
    fn reinit(&mut self) -> std::io::Result<()>;

    /// Encode the provided input buffer into the provided output buffer.
    ///
    /// Returns `true` iff the end of the input stream has been reached.
    fn decode(
        &mut self,
        input: &mut PartialBuffer<impl AsRef<[u8]>>,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> std::io::Result<bool>;

    /// Flush the internal buffers into the provided output buffer.
    ///
    /// Returns `true` iff the internal buffers have been completely flushed.
    fn flush(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> std::io::Result<bool>;

    /// Finish the decoding into the provided output buffer.
    ///
    /// Returns `true` iff the internal buffers have been completely flushed.
    fn finish(
        &mut self,
        output: &mut PartialBuffer<impl AsRef<[u8]> + AsMut<[u8]>>,
    ) -> std::io::Result<bool>;
}
