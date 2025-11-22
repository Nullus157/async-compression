use std::io::Result;

use async_compression::tokio::write::{PpmdDecoder, PpmdEncoder};
use async_compression::Level;
use tokio::io::AsyncWriteExt as _; // for `write_all` and `shutdown`

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let data = b"example-ppmd";
    let compressed_data = compress(data).await?;
    let de_compressed_data = decompress(&compressed_data).await?;
    assert_eq!(de_compressed_data, data);
    println!("{}", String::from_utf8(de_compressed_data).unwrap());
    Ok(())
}

async fn compress(in_data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = PpmdEncoder::with_quality(Vec::new(), Level::Fastest);
    encoder.write_all(in_data).await?;
    encoder.shutdown().await?;
    Ok(encoder.into_inner())
}

async fn decompress(in_data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = PpmdDecoder::new(Vec::new());
    decoder.write_all(in_data).await?;
    decoder.shutdown().await?;
    Ok(decoder.into_inner())
}
