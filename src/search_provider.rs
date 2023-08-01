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
use crate::page_source::ExtractedPage;
use crate::vector::{Embedding, EM_LEN};

#[derive(Debug)]
pub struct SearchResult {
    pub distance: f32,
    pub url: String,
    pub title: String,
}

struct PageData {
    url: String,
    title: String,
    text: String,
}

pub struct SearchProvider {
    model: SentenceEmbeddingsModel,
    index: UniquePtr<Index>,

    // Temp storage
    data: Vec<PageData>,
}

const INDEX_OPTIONS: IndexOptions = IndexOptions {
    dimensions: EM_LEN,
    metric: MetricKind::IP,
    quantization: ScalarKind::F8,
    connectivity: 0,
    expansion_add: 0,
    expansion_search: 0,
};

impl SearchProvider {
    pub fn new() -> Result<SearchProvider, anyhow::Error> {
        let start = Instant::now();
        print!("Loading model...");
        let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
            .with_device(tch::Device::Cpu)
            .create_model()?;

        let duration = start.elapsed();
        println!(" {} ms", duration.as_millis());

        let index = new_index(&INDEX_OPTIONS).unwrap();

        Ok(SearchProvider {
            model,
            // document_embeddings,
            index,
            data: Vec::new(),
        })
    }

    pub fn load(warc_dir: &str) -> Result<SearchProvider, anyhow::Error> {
        let start = Instant::now();

        print!("Loading model...");

        let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
            .with_device(tch::Device::Cpu)
            .create_model()?;

        let duration = start.elapsed();
        println!(" {} ms", duration.as_millis());

        let mut data = Vec::new();
        let document_embeddings = DocumentEmbeddings::load(&warc_dir)?;
        for page in 0..document_embeddings.files() {
            for entry in 0..document_embeddings.entries(page) {
                let url = str::from_utf8(document_embeddings.url(page, entry))?.to_string();
                let title = str::from_utf8(document_embeddings.title(page, entry))?.to_string();
                let text = String::new();

                data.push(PageData { url, title, text })
            }
        }

        let total_documents = document_embeddings.len();

        let index = new_index(&INDEX_OPTIONS).unwrap();

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
            // document_embeddings,
            index,
            data,
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
            result.push(SearchResult {
                distance,
                url: self.data[id as usize].url.clone(),
                title: self.data[id as usize].title.clone(),
            });
        }
        println!("Search completed in {} us", duration.as_micros(),);

        Ok(result)
    }

    pub fn insert(&mut self, page: ExtractedPage) -> Result<(), anyhow::Error> {
        println!("Inserting {}", page.url);

        let q = &self.model.encode(&[page.combined]).unwrap()[0];
        if self.index.capacity() == self.data.len() {
            self.index.reserve(self.data.len() + 1000)?;
        }
        self.index.add(self.data.len() as u64, q)?;
        self.data.push(PageData {
            url: page.url,
            title: page.title,
            text: page.text,
        });
        Ok(())
    }
}
