use futures::stream::{Stream, StreamExt as _};
use futures_test::stream::StreamTestExt as _;
use proptest_derive::Arbitrary;

#[derive(Arbitrary, Debug, Clone)]
pub struct InputStream(Vec<Vec<u8>>);

impl InputStream {
    pub fn as_ref(&self) -> &[Vec<u8>] {
        &self.0
    }

    pub fn stream(&self) -> impl Stream<Item = Vec<u8>> {
        // The resulting stream here will interleave empty chunks before and after each chunk, and
        // then interleave a `Poll::Pending` between each yielded chunk, that way we test the
        // handling of these two conditions in every point of the tested stream.
        futures::stream::iter(
            self.0
                .clone()
                .into_iter()
                .flat_map(|bytes| vec![vec![], bytes])
                .chain(Some(vec![])),
        )
        .interleave_pending()
    }

    pub fn bytes_05_stream(&self) -> impl Stream<Item = std::io::Result<bytes_05::Bytes>> {
        self.stream()
            .map(bytes_05::Bytes::from)
            .map(std::io::Result::Ok)
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.0.iter().flatten().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.0.iter().map(Vec::len).sum()
    }
}

// This happens to be the only dimension we're using
impl From<[[u8; 3]; 2]> for InputStream {
    fn from(input: [[u8; 3]; 2]) -> InputStream {
        InputStream(vec![Vec::from(&input[0][..]), Vec::from(&input[1][..])])
    }
}

impl From<Vec<Vec<u8>>> for InputStream {
    fn from(input: Vec<Vec<u8>>) -> InputStream {
        InputStream(input)
    }
}
