use async_compression::tokio::bufread::ZstdDecoder;

#[tokio::test]
async fn multiple_frames() {
    let llvm_as = tokio::fs::File::open("tests/artifacts/llvm-as.zstd")
        .await
        .unwrap();
    let buffered = tokio::io::BufReader::new(llvm_as);
    let mut decoder = ZstdDecoder::new(buffered);
    let mut out = Vec::new();
    tokio::io::copy(&mut decoder, &mut out).await.unwrap();
    assert_eq!(out.len(), 4401672)
}
