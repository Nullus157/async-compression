use async_compression::tokio::bufread::ZstdDecoder;
use async_compression::tokio::write::GzipEncoder;

use std::io::Result;
use tokio::io::stderr;
use tokio::io::stdin;
use tokio::io::stdout;
use tokio::io::AsyncReadExt as _; // for `read_to_end`
use tokio::io::AsyncWriteExt as _; // for `write_all` and `shutdown`
use tokio::io::BufReader;

// Run this example by running the following in the terminal:
// ```
// echo 'example' | zstd | cargo run --example zstd_gzip --features="all" | gunzip -c                                                                                                                                                    ─╯
// ```

#[tokio::main]
async fn main() -> Result<()> {
    // Read zstd encoded data from stdin and decode
    let mut reader = ZstdDecoder::new(BufReader::new(stdin()));
    let mut x: Vec<u8> = vec![];
    reader.read_to_end(&mut x).await?;

    // print to stderr the length of the decoded data
    let mut error = stderr();
    error.write_all(x.len().to_string().as_bytes()).await?;
    error.shutdown().await?;

    // print to stdin encoded gzip data
    let mut writer = GzipEncoder::new(stdout());
    writer.write_all(&x).await?;
    writer.shutdown().await?;

    // flush stdout
    let mut res = writer.into_inner();
    res.flush().await?;

    Ok(())
}
