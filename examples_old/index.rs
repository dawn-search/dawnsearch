use std::{env, fs::File};

use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};

use dawnsearch::warc::extract_records_and_add_to_index;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let warc_dir = &args[1];

    unsafe {
        torch_sys::dummy_cuda_dependency();
    }
    println!("CUDA: {}", tch::Cuda::is_available());

    let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
        .create_model()?;

    let mut numfiles = 0;
    for path in std::fs::read_dir(warc_dir).unwrap() {
        numfiles += 1;
        let filename = path.unwrap().path();
        let s = filename.to_string_lossy();
        if !s.ends_with(".warc.gz") {
            continue;
        }
        eprintln!("{}\t{}", numfiles, s);
        let mut file = File::open(&filename).unwrap();
        extract_records_and_add_to_index(&mut file, &filename, &model)?;
    }

    Ok(())
}
