use std::{env, fs::File, path::Path};

use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};

use arecibo::warc::extract_records_and_add_to_index;
use url::Url;

/**
 * Usage: index_urls ../wat https://...
 */
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let dir = &args[1];
    let file_name = &args[2];

    unsafe {
        torch_sys::dummy_cuda_dependency();
    }
    println!("CUDA: {}", tch::Cuda::is_available());

    let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
        .create_model()?;

    if !file_name.ends_with(".warc.gz") {
        panic!("File needs to be .warc.gz");
    }

    let mut url = "https://data.commoncrawl.org/".to_string();
    url.push_str(file_name);

    let u = Url::parse(&url).unwrap();
    let filename = u.path().split("/").last().unwrap();

    let output_path = Path::new(dir);
    let pb = output_path.join(filename);
    println!("Writing to {}", pb.to_str().unwrap());

    let mut response = reqwest::blocking::get(url)?;
    extract_records_and_add_to_index(&mut response, &pb, &model)?;

    Ok(())
}
