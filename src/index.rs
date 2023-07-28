use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::{self, collections::HashSet};
use std::{env, str};

use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};
use warc::EM_LEN;

use crate::warc::extract_records_and_add_to_index;

#[path = "warc.rs"]
mod warc;

fn lines_from_file(filename: impl AsRef<Path>) -> HashSet<String> {
    let file = File::open(filename).expect("no such file");
    let buf = BufReader::new(file);
    let mut result = HashSet::new();
    for line in buf.lines() {
        let l = line.ok().unwrap();
        result.insert(l);
    }
    result
}

pub fn find_embedding(
    embeddings: &HashMap<&str, &[f32; EM_LEN]>,
    s: &str,
    embedding: &mut [f32; 300],
) -> f32 {
    let mut embedding_scratch: [f32; EM_LEN] = [0.0; EM_LEN];
    let mut total = 0;
    let mut found = 0;
    for word in s.split(|c: char| !c.is_alphanumeric()) {
        if word.len() == 0 {
            continue;
        }
        total += 1;
        match embeddings.get(word) {
            Some(e) => {
                found += 1;
                for (i, v) in e.iter().enumerate() {
                    embedding_scratch[i] += *v;
                }
            }
            None => {}
        }
    }
    if found == 0 {
        embedding.fill(0.0); // Average
        return 0.0;
    }
    for (i, v) in embedding_scratch.iter().enumerate() {
        embedding[i] = v / found as f32;
    }
    found as f32 / total as f32
}

pub fn distance(a: &[f32; EM_LEN], b: &[f32; EM_LEN]) -> f32 {
    let mut result: f32 = 0.0;
    for (i, aa) in a.iter().enumerate() {
        result += (*aa as f32 - b[i] as f32).powf(2.0);
    }
    result
}

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
        extract_records_and_add_to_index(&filename, &model)?;
    }

    Ok(())
}
