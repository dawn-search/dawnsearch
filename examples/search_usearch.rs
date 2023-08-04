use std::io::{self, BufRead, Write};
use std::iter::zip;
use std::time::Instant;
use std::{self, fs};
use std::{str, usize};

use dawnsearch::util::default_progress_bar;
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};
use std::env;
use usearch::ffi::{new_index, IndexOptions, MetricKind, ScalarKind};

use dawnsearch::document_embeddings::DocumentEmbeddings;
use dawnsearch::vector::{Embedding, EM_LEN};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let warc_dir = &args[1];

    let start = Instant::now();

    print!("Loading model...");

    let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
        .with_device(tch::Device::Cpu)
        .create_model()?;

    let duration = start.elapsed();
    println!(" {} ms", duration.as_millis());

    let document_embeddings = DocumentEmbeddings::load(&warc_dir)?;
    let total_documents = document_embeddings.len();

    let options = IndexOptions {
        dimensions: EM_LEN,
        metric: MetricKind::IP,
        quantization: ScalarKind::F8,
        connectivity: 0,
        expansion_add: 0,
        expansion_search: 0,
    };

    let index = new_index(&options).unwrap();

    let index_path = "index.usearch";
    if !fs::metadata(index_path).is_ok() || !index.view(index_path).is_ok() {
        println!("Recalculating index...");
        index.reserve(document_embeddings.len())?;

        let progress = default_progress_bar(total_documents);
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
        index.save(index_path)?;
    } else {
        println!("Loaded index {}", index_path);
    }

    let stdin = io::stdin();
    println!("Ready. Please enter your search: ");
    println!("");
    print!("> ");
    io::stdout().flush()?;
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
        let fraction = total_documents as f32 / (80000.0 * 7000.0);
        println!("");
        println!(
            "Searched {} pages in {} us ({:.2}% of the common crawl database)",
            total_documents,
            duration.as_micros(),
            fraction * 100.0
        );
        println!("");
        print!("> ");
        io::stdout().flush()?;
    }

    Ok(())
}
