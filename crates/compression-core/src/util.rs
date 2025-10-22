pub const fn _assert_send<T: Send>() {}
pub const fn _assert_sync<T: Sync>() {}

#[derive(Debug, Default)]
pub struct PartialBuffer<B> {
    buffer: B,
    index: usize,
}

impl<B: AsRef<[u8]>> PartialBuffer<B> {
    pub fn new(buffer: B) -> Self {
        Self { buffer, index: 0 }
    }

    pub fn written(&self) -> &[u8] {
        &self.buffer.as_ref()[..self.index]
    }

    /// Convenient method for `.writen().len()`
    pub fn written_len(&self) -> usize {
        self.index
    }

    pub fn unwritten(&self) -> &[u8] {
        &self.buffer.as_ref()[self.index..]
    }

    pub fn advance(&mut self, amount: usize) {
        self.index += amount;
        debug_assert!(self.index <= self.buffer.as_ref().len());
    }

    pub fn get_mut(&mut self) -> &mut B {
        &mut self.buffer
    }

    pub fn into_inner(self) -> B {
        self.buffer
    }

    pub fn reset(&mut self) {
        self.index = 0;
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> PartialBuffer<B> {
    pub fn unwritten_mut(&mut self) -> &mut [u8] {
        &mut self.buffer.as_mut()[self.index..]
    }

    pub fn copy_unwritten_from<C: AsRef<[u8]>>(&mut self, other: &mut PartialBuffer<C>) -> usize {
        let len = self.unwritten().len().min(other.unwritten().len());

        self.unwritten_mut()[..len].copy_from_slice(&other.unwritten()[..len]);

        self.advance(len);
        other.advance(len);
        len
    }
}

impl<B: AsRef<[u8]> + Default> PartialBuffer<B> {
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> From<B> for PartialBuffer<B> {
    fn from(buffer: B) -> Self {
        Self::new(buffer)
    }
}

/// Write buffer for compression-codecs.
///
/// Currently it only supports initialized buffer, but will support uninitialized
/// buffer soon.
///
/// # Layout
///
/// |                                       buffer                                    |
/// | written and initialized | unwritten but initialized | unwritten and uninitialized
#[derive(Debug)]
pub struct WriteBuffer<'a> {
    buffer: &'a mut [u8],
    index: usize,
}

impl<'a> WriteBuffer<'a> {
    pub fn new_initialized(buffer: &'a mut [u8]) -> Self {
        Self { buffer, index: 0 }
    }

    pub fn written(&self) -> &[u8] {
        &self.buffer[..self.index]
    }

    /// Convenient method for `.writen().len()`
    pub fn written_len(&self) -> usize {
        self.index
    }

    /// Buffer has no spare space to write any data
    pub fn has_no_spare_space(&self) -> bool {
        self.index == self.buffer.len()
    }

    /// Initialize all uninitialized, unwritten part to initialized, unwritten part
    pub fn initialize_unwritten(&mut self) {}

    /// Return initialized but unwritten part.
    pub fn unwritten_initialized_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[self.index..]
    }

    /// Advance written index within initialized part.
    ///
    /// Note that try to advance into uninitialized part would panic.
    pub fn advance(&mut self, amount: usize) {
        self.index += amount;
        debug_assert!(self.index <= self.buffer.len());
    }

    pub fn reset(&mut self) {
        self.index = 0;
    }

    pub fn copy_unwritten_from<C: AsRef<[u8]>>(&mut self, other: &mut PartialBuffer<C>) -> usize {
        let len = self
            .unwritten_initialized_mut()
            .len()
            .min(other.unwritten().len());

        self.unwritten_initialized_mut()[..len].copy_from_slice(&other.unwritten()[..len]);

        self.advance(len);
        other.advance(len);
        len
    }
}
