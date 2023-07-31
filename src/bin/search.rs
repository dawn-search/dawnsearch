use std::io::{self, BufRead};
use std::str;
use std::time::Instant;
use std::{self};

use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};
use std::env;

use arecibo::document_embeddings::DocumentEmbeddings;
use arecibo::vector::{Distance, EM_LEN};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let warc_dir = &args[1];

    let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
        .with_device(tch::Device::Cpu)
        .create_model()?;

    let document_embeddings = DocumentEmbeddings::load(&warc_dir)?;

    struct ScoredBook {
        score: f32,
        file: usize,
        entry: usize,
    }

    let stdin = io::stdin();
    eprint!("> ");
    for q in stdin.lock().lines() {
        println!("");
        let query = q.unwrap();

        let q = &model.encode(&[query]).unwrap()[0];
        let query_embedding: &[f32; EM_LEN] = q.as_slice().try_into().unwrap();

        let mut results: Vec<ScoredBook> = Vec::new();

        let start = Instant::now();

        let mut searched_pages_count = 0;
        for page in 0..document_embeddings.files() {
            for entry in 0..document_embeddings.entries(page) {
                searched_pages_count += 1;
                let p = document_embeddings.entry(page, entry);
                let score = p.vector.distance(query_embedding);

                if results.len() < 10 {
                    results.push(ScoredBook {
                        file: page,
                        score,
                        entry,
                    });
                    continue;
                }
                if score < results[9].score {
                    results[9] = ScoredBook {
                        file: page,
                        score,
                        entry,
                    };
                    results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
                }
            }
        }

        //////////////

        for r in results {
            let url: &[u8] = document_embeddings.url(r.file, r.entry);
            let title: &[u8] = document_embeddings.title(r.file, r.entry);
            println!(
                "{}: {} - {}",
                r.score,
                unsafe { str::from_utf8_unchecked(title) },
                unsafe { str::from_utf8_unchecked(url) },
            );
        }
        let duration = start.elapsed();
        let fraction = searched_pages_count as f32 / (80000.0 * 7000.0);
        println!("");
        println!(
            "Searched {} pages in {:.1} ms ({:.2}% of the common crawl database)",
            searched_pages_count,
            duration.as_millis(),
            fraction * 100.0
        );
        println!("");
        eprint!("> ");
    }

    Ok(())
}
