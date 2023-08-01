use std::iter::zip;
use std::time::Instant;
use std::{self, fs};
use std::{str, usize};

use cxx::UniquePtr;
use indicatif::{ProgressBar, ProgressStyle};
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType,
};
use usearch::ffi::{new_index, Index, IndexOptions, MetricKind, ScalarKind};

use crate::document_embeddings::DocumentEmbeddings;
use crate::vector::{Embedding, EM_LEN};

#[derive(Debug)]
pub struct SearchResult {
    pub distance: f32,
    pub url: String,
    pub title: String,
}

pub struct SearchProvider {
    model: SentenceEmbeddingsModel,
    document_embeddings: DocumentEmbeddings,
    index: UniquePtr<Index>,
}

impl SearchProvider {
    pub fn load(warc_dir: &str) -> Result<SearchProvider, anyhow::Error> {
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

            let progress = ProgressBar::new(total_documents as u64);
            progress.set_style(
                ProgressStyle::with_template("[{elapsed_precise}] {bar}{pos:>7}/{len:7} {msg}")
                    .unwrap(),
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
            index.save(index_path)?;
        } else {
            println!("Loaded index {}", index_path);
        }
        Ok(SearchProvider {
            model,
            document_embeddings,
            index,
        })
    }

    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, anyhow::Error> {
        let mut result = Vec::new();

        let q = &self.model.encode(&[query]).unwrap()[0];
        let query_embedding: &Embedding<f32> = q.as_slice().try_into().unwrap();

        let start = Instant::now();

        // Read back the tags
        let results = self.index.search(query_embedding, 20).unwrap();

        let duration = start.elapsed();

        for (distance, id) in zip(results.distances, results.labels) {
            let (file, entry) = self.document_embeddings.linear_to_segmented(id as usize);

            // let e = document_embeddings.entry(file, entry);
            let url: &[u8] = self.document_embeddings.url(file, entry);
            let title: &[u8] = self.document_embeddings.title(file, entry);
            // println!(
            //     "{:.2}: {} - {}",
            //     distance,
            //     unsafe { str::from_utf8_unchecked(title) },
            //     unsafe { str::from_utf8_unchecked(url) }
            // );
            result.push(SearchResult {
                distance,
                url: str::from_utf8(url)?.to_owned(),
                title: str::from_utf8(title)?.to_owned(),
            });
        }
        println!("Search completed in {} us", duration.as_micros(),);

        Ok(result)
    }
}
