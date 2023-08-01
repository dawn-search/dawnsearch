use std::{
    env,
    fs::File,
    io::{self, Write},
    path::Path,
};

use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};

use arecibo::warc::extract_records_and_add_to_index;
use aws_sdk_s3 as s3;
use tokio::join;
use url::Url;

/**
 * Usage: index_urls ../wat https://...
 */
#[::tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let dir = &args[1];
    let key = &args[2];

    unsafe {
        torch_sys::dummy_cuda_dependency();
    }
    println!("CUDA: {}", tch::Cuda::is_available());

    let model = tokio::task::spawn_blocking(move || {
        SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
            .create_model()
            .unwrap()
    })
    .await?;

    if !key.ends_with(".warc.gz") {
        panic!("File needs to be .warc.gz");
    }

    let mut url_string = "https://data.commoncrawl.org/".to_string();
    url_string.push_str(key);

    let url = Url::parse(&url_string).unwrap();
    let only_file_name = url.path().split("/").last().unwrap();

    let output_path = Path::new(dir);
    let pb = output_path.join(only_file_name);
    println!("Writing to {}", pb.to_str().unwrap());

    // let mut response = reqwest::blocking::get(url_string)?;

    let config = ::aws_config::load_from_env().await;
    let client = s3::Client::new(&config);

    let response_async = client
        .get_object()
        .bucket("commoncrawl")
        .key(key)
        .send()
        .await?
        .body
        .into_async_read();
    let _ = tokio::task::spawn_blocking(move || {
        let mut response_sync = tokio_util::io::SyncIoBridge::new(response_async);
        extract_records_and_add_to_index(&mut response_sync, &pb, &model).unwrap();
    })
    .await;

    println!("Shutting down");

    Ok(())
}
