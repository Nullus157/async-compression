#[allow(unused)]
use futures::{executor::block_on, io::AsyncReadExt, stream::StreamExt};

#[macro_use]
mod utils;

test_cases!(xz);

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "stream")]
fn stream_multiple_members_with_padding() {
    let compressed = [
        utils::xz::sync::compress(&[1, 2, 3, 4, 5, 6]),
        vec![0, 0, 0, 0],
        utils::xz::sync::compress(&[6, 5, 4, 3, 2, 1]),
        vec![0, 0, 0, 0],
    ]
    .join(&[][..]);

    let stream = utils::InputStream::from(vec![compressed]);

    let mut decoder = utils::xz::stream::Decoder::new(stream.stream());
    decoder.multiple_members(true);
    let output = utils::prelude::stream_to_vec(decoder);

    assert_eq!(output, &[1, 2, 3, 4, 5, 6, 6, 5, 4, 3, 2, 1][..]);
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "stream")]
fn stream_multiple_members_with_invalid_padding() {
    let compressed = [
        utils::xz::sync::compress(&[1, 2, 3, 4, 5, 6]),
        vec![0, 0, 0],
        utils::xz::sync::compress(&[6, 5, 4, 3, 2, 1]),
        vec![0, 0, 0, 0],
    ]
    .join(&[][..]);

    let stream = utils::InputStream::from(vec![compressed]);

    let mut decoder = utils::xz::stream::Decoder::new(stream.stream());
    decoder.multiple_members(true);

    assert!(block_on(decoder.next()).unwrap().is_err());
    assert!(block_on(decoder.next()).is_none());
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "futures-bufread")]
fn bufread_multiple_members_with_padding() {
    let compressed = [
        utils::xz::sync::compress(&[1, 2, 3, 4, 5, 6]),
        vec![0, 0, 0, 0],
        utils::xz::sync::compress(&[6, 5, 4, 3, 2, 1]),
        vec![0, 0, 0, 0],
    ]
    .join(&[][..]);

    let stream = utils::InputStream::from(vec![compressed]);

    let mut decoder = utils::xz::futures::bufread::Decoder::new(stream.reader());
    decoder.multiple_members(true);
    let output = utils::prelude::async_read_to_vec(decoder);

    assert_eq!(output, &[1, 2, 3, 4, 5, 6, 6, 5, 4, 3, 2, 1][..]);
}

#[test]
#[ntest::timeout(1000)]
#[cfg(feature = "futures-bufread")]
fn bufread_multiple_members_with_invalid_padding() {
    let compressed = [
        utils::xz::sync::compress(&[1, 2, 3, 4, 5, 6]),
        vec![0, 0, 0],
        utils::xz::sync::compress(&[6, 5, 4, 3, 2, 1]),
        vec![0, 0, 0, 0],
    ]
    .join(&[][..]);

    let stream = utils::InputStream::from(vec![compressed]);

    let mut decoder = utils::xz::futures::bufread::Decoder::new(stream.reader());
    decoder.multiple_members(true);

    let mut output = Vec::new();
    assert!(block_on(decoder.read_to_end(&mut output)).is_err());
}
