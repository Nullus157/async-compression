pub fn _assert_send<T: Send>() {}
pub fn _assert_sync<T: Sync>() {}

#[derive(Debug)]
pub(crate) struct PartialBuffer<B: AsRef<[u8]>> {
    buffer: B,
    index: usize,
}

impl<B: AsRef<[u8]>> PartialBuffer<B> {
    pub(crate) fn new(buffer: B) -> Self {
        Self { buffer, index: 0 }
    }

    pub(crate) fn written(&self) -> &[u8] {
        &self.buffer.as_ref()[..self.index]
    }

    pub(crate) fn unwritten(&self) -> &[u8] {
        &self.buffer.as_ref()[self.index..]
    }

    pub(crate) fn advance(&mut self, amount: usize) {
        self.index += amount;
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]> + Default> PartialBuffer<B> {
    pub(crate) fn unwritten_mut(&mut self) -> &mut [u8] {
        &mut self.buffer.as_mut()[self.index..]
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]> + Default> PartialBuffer<B> {
    pub(crate) fn take(&mut self) -> Self {
        std::mem::replace(self, Self::new(B::default()))
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> From<B> for PartialBuffer<B> {
    fn from(buffer: B) -> Self {
        Self::new(buffer)
    }
}
