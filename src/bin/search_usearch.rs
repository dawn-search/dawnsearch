use std::io::{self, BufRead};
use std::iter::zip;
use std::time::Instant;
use std::{self};
use std::{str, usize};

use indicatif::{ProgressBar, ProgressStyle};
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};
use std::env;
use usearch::ffi::{new_index, IndexOptions, MetricKind, ScalarKind};

use arecibo::document_embeddings::DocumentEmbeddings;
use arecibo::vector::{Embedding, EM_LEN};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let warc_dir = &args[1];

    let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
        .with_device(tch::Device::Cpu)
        .create_model()?;

    let document_embeddings = DocumentEmbeddings::load(&warc_dir)?;

    let options = IndexOptions {
        dimensions: EM_LEN,
        metric: MetricKind::IP,
        quantization: ScalarKind::F8,
        connectivity: 0,
        expansion_add: 0,
        expansion_search: 0,
    };

    let index = new_index(&options).unwrap();

    index.reserve(document_embeddings.len())?;

    let progress = ProgressBar::new(document_embeddings.len() as u64);
    progress.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] {bar}{pos:>7}/{len:7} {msg}").unwrap(),
    );
    let mut searched_pages_count = 0;
    for page in 0..document_embeddings.files() {
        for entry in 0..document_embeddings.entries(page) {
            progress.set_position(searched_pages_count);
            let p = document_embeddings.entry(page, entry);
            index.add(searched_pages_count, &p.vector)?;
            searched_pages_count += 1;
        }
    }
    progress.finish();

    let stdin = io::stdin();
    eprint!("> ");
    let mut previous_query = String::new();
    for q in stdin.lock().lines() {
        println!("");
        let mut query = q.unwrap();
        if query.is_empty() {
            query = previous_query.clone();
        } else {
            previous_query = query.clone();
        }

        let q = &model.encode(&[query]).unwrap()[0];
        let query_embedding: &Embedding<f32> = q.as_slice().try_into().unwrap();

        let start = Instant::now();

        // Read back the tags
        let results = index.search(query_embedding, 10).unwrap();

        let duration = start.elapsed();

        let mut count = 0;
        for (distance, id) in zip(results.distances, results.labels) {
            count += 1;
            if count > 10 {
                break;
            }
            let (file, entry) = document_embeddings.linear_to_segmented(id as usize);
            // let e = document_embeddings.entry(file, entry);
            let url: &[u8] = document_embeddings.url(file, entry);
            let title: &[u8] = document_embeddings.title(file, entry);
            println!(
                "{:.2}: {} - {}",
                distance,
                unsafe { str::from_utf8_unchecked(title) },
                unsafe { str::from_utf8_unchecked(url) }
            );
        }
        let fraction = searched_pages_count as f32 / (80000.0 * 7000.0);
        println!("");
        println!(
            "Searched {} pages in {} us ({:.2}% of the common crawl database)",
            searched_pages_count,
            duration.as_micros(),
            fraction * 100.0
        );
        println!("");
        eprint!("> ");
    }

    Ok(())
}
