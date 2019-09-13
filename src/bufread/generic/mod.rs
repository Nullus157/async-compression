mod encoder;
mod decoder;

pub use self::{encoder::Encoder, decoder::Decoder};

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

