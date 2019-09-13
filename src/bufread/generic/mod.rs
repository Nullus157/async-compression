mod decoder;
mod encoder;

pub use self::{decoder::Decoder, encoder::Encoder};

#[derive(Debug)]
struct PartialBuffer<B: AsRef<[u8]> + AsMut<[u8]>> {
    buffer: B,
    index: usize,
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> PartialBuffer<B> {
    fn new(buffer: B) -> Self {
        Self { buffer, index: 0 }
    }

    fn written(&self) -> &[u8] {
        &self.buffer.as_ref()[..self.index]
    }

    fn unwritten(&mut self) -> &mut [u8] {
        &mut self.buffer.as_mut()[self.index..]
    }

    fn advance(&mut self, amount: usize) {
        self.index += amount;
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]> + Default> PartialBuffer<B> {
    fn take(&mut self) -> Self {
        std::mem::replace(self, Self::new(B::default()))
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> From<B> for PartialBuffer<B> {
    fn from(buffer: B) -> Self {
        Self::new(buffer)
    }
}
