use std::iter::FromIterator;

mod utils;

/// Splits the input bytes into the first 10 bytes, the rest and the last 8 bytes, taking apart the
/// 3 parts of compressed gzip data.
fn split(mut input: Vec<u8>) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    assert!(input.len() >= 18);

    let mut body = input.split_off(10);
    let header = input;
    let footer = body.split_off(body.len() - 8);

    (header, body, footer)
}

#[test]
fn gzip_stream_compress() {
    let input = utils::InputStream::from([[1, 2, 3], [4, 5, 6]]);

    let compressed = utils::gzip_stream_compress(input.stream());
    let output = utils::gzip_decompress(&compressed);

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn gzip_stream_compress_large() {
    let input = vec![
        Vec::from_iter((0..20_000).map(|_| rand::random())),
        Vec::from_iter((0..20_000).map(|_| rand::random())),
    ];
    let input = utils::InputStream::from(input);

    let compressed = utils::gzip_stream_compress(input.stream());
    let output = utils::gzip_decompress(&compressed);

    assert_eq!(output, input.bytes());
}

#[test]
fn gzip_stream_decompress_single_chunk() {
    let compressed = utils::gzip_compress(&[1, 2, 3, 4, 5, 6][..]);

    // The entirety in one chunk
    let stream = utils::InputStream::from(vec![compressed]);
    let output = utils::gzip_stream_decompress(stream.stream());

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn gzip_stream_decompress_segmented() {
    let (header, body, footer) = split(utils::gzip_compress(&[1, 2, 3, 4, 5, 6][..]));

    // Header, body and footer in separate chunks, similar to how `GzipStream` outputs it.
    let stream = utils::InputStream::from(vec![header, body, footer]);
    let output = utils::gzip_stream_decompress(stream.stream());

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn gzip_stream_decompress_split() {
    let (header, body, footer) = split(utils::gzip_compress(&[1, 2, 3, 4, 5, 6][..]));

    // Header, body and footer each split across multiple chunks, no mixing
    let stream = utils::InputStream::from(vec![
        Vec::from(&header[0..5]),
        Vec::from(&header[5..10]),
        Vec::from(&body[0..body.len() / 2]),
        Vec::from(&body[body.len() / 2..]),
        Vec::from(&footer[0..4]),
        Vec::from(&footer[4..8]),
    ]);

    let output = utils::gzip_stream_decompress(stream.stream());

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn gzip_stream_decompress_split_mixed() {
    let (header, body, footer) = split(utils::gzip_compress(&[1, 2, 3, 4, 5, 6][..]));

    // Header, body and footer split across multiple chunks and mixed together
    let stream = utils::InputStream::from(vec![
        Vec::from(&header[0..5]),
        Vec::from_iter(
            header[5..10]
                .iter()
                .chain(body[0..body.len() / 2].iter())
                .cloned(),
        ),
        Vec::from_iter(
            body[body.len() / 2..]
                .iter()
                .chain(footer[0..4].iter())
                .cloned(),
        ),
        Vec::from(&footer[4..8]),
    ]);

    let output = utils::gzip_stream_decompress(stream.stream());

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn gzip_stream_decompress_empty() {
    let compressed = utils::gzip_compress(&[][..]);

    let stream = utils::InputStream::from(vec![compressed]);
    let output = utils::gzip_stream_decompress(stream.stream());

    assert_eq!(output, vec![]);
}

#[test]
fn gzip_stream_decompress_large() {
    let compressed = utils::gzip_compress(&[1; 20_000]);

    let stream = utils::InputStream::from(vec![compressed]);
    let output = utils::gzip_stream_decompress(stream.stream());

    assert_eq!(output, &[1; 20_000][..]);
}
