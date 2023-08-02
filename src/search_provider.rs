use std::iter::zip;
use std::time::Instant;
use std::{self, fs};
use std::{str, usize};

use anyhow::bail;
use cxx::UniquePtr;
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType,
};
use tokio_util::sync::CancellationToken;
use usearch::ffi::{new_index, Index, IndexOptions, MetricKind, ScalarKind};

use crate::page_source::ExtractedPage;
use crate::util::default_progress_bar;
use crate::vector::{
    bytes_to_embedding, is_normalized, vector_embedding_to_bytes, Embedding, EM_LEN,
};

// Remove the index when you change any of these values!
const INDEX_OPTIONS: IndexOptions = IndexOptions {
    dimensions: EM_LEN,
    metric: MetricKind::IP,
    quantization: ScalarKind::F32,
    connectivity: 0,
    expansion_add: 0,
    expansion_search: 0,
};

#[derive(Debug)]
pub struct SearchResult {
    pub pages: Vec<FoundPage>,
    pub pages_searched: usize,
}

#[derive(Debug)]
pub struct FoundPage {
    pub distance: f32,
    pub url: String,
    pub title: String,
    pub text: String,
    pub embedding: Vec<f32>,
}

pub struct SearchProvider {
    model: SentenceEmbeddingsModel,
    index: UniquePtr<Index>,

    sqlite: rusqlite::Connection,

    shutdown_token: CancellationToken,
}

impl SearchProvider {
    pub fn new(shutdown_token: CancellationToken) -> Result<SearchProvider, anyhow::Error> {
        let start = Instant::now();
        print!("[Search Provider] Loading model...");
        let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
            .with_device(tch::Device::Cpu)
            .create_model()?;

        let duration = start.elapsed();
        println!(" {} ms", duration.as_millis());

        // Database
        let sqlite = rusqlite::Connection::open("arecibo.sqlite")?;

        // Create DB structure
        sqlite.execute(
            "CREATE TABLE IF NOT EXISTS page (
                id INTEGER PRIMARY KEY,
                url TEXT NOT NULL,
                title TEXT NOT NULL,
                text INTEGER NOT NULL,
                embedding BLOB NOT NULL
            )",
            (),
        )?;
        sqlite.execute(
            "
            CREATE INDEX IF NOT EXISTS find_by_url on page(url)
        ",
            (),
        )?;

        // Index
        let index = new_index(&INDEX_OPTIONS).unwrap();

        let mut search_provider = SearchProvider {
            model,
            index,
            sqlite,
            shutdown_token: shutdown_token.clone(),
        };

        let index_path = "index.usearch";
        if !fs::metadata(index_path).is_ok() || !search_provider.index.load(index_path).is_ok() {
            search_provider.fill_index_from_db()?;
            search_provider.index.save(index_path)?;
        } else {
            println!("[Search Provider] Loaded {}", index_path);
        }

        search_provider.verify()?;

        Ok(search_provider)
    }

    fn fill_index_from_db(&mut self) -> Result<(), anyhow::Error> {
        // Fill from DB
        let count = self.page_count()?;
        let progress = default_progress_bar(count);
        progress.set_prefix("Rebuilding index");

        self.index.reserve(count)?;

        let mut s = self
            .sqlite
            .prepare("SELECT id, embedding FROM page")
            .unwrap();
        let mut qq = s.query(()).unwrap();
        while let Some(r) = qq.next().unwrap() {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            progress.inc(1);
            let id: u64 = r.get(0).unwrap();
            let embedding: Vec<u8> = r.get(1).unwrap();
            let q: &[f32] = unsafe { bytes_to_embedding(embedding.as_slice().try_into()?)? };

            self.index.add(id, q).unwrap();
        }
        progress.finish_and_clear();
        Ok(())
    }

    fn page_count(&mut self) -> Result<usize, anyhow::Error> {
        let count = self
            .sqlite
            .query_row("SELECT count(*) FROM page", (), |row| {
                row.get::<_, usize>(0)
            })?;
        Ok(count)
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        println!("[Search Provider] Shutting down...");
        self.index.save("index.usearch")?;
        println!("[Search Provider] Shutdown completed");
        Ok(())
    }

    pub fn search(&self, query: &str) -> Result<SearchResult, anyhow::Error> {
        let mut pages = Vec::new();

        let q = &self.model.encode(&[query])?[0];
        let query_embedding: &Embedding<f32> = q.as_slice().try_into()?;

        if !is_normalized(query_embedding) {
            bail!("Search vector is not normalized");
        }

        let start = Instant::now();

        // Read back the tags
        let results = self.index.search(query_embedding, 20)?;

        let duration = start.elapsed();

        let mut s = self
            .sqlite
            .prepare("SELECT id, url, title, text, embedding FROM page WHERE id  = ?1")?;
        for (distance, id) in zip(results.distances, results.labels) {
            let mut qq = s.query(&[&id])?;
            if let Some(r) = qq.next()? {
                let _id: u64 = r.get(0)?;
                let url = r.get(1)?;
                let title = r.get(2)?;
                let text: String = r.get(3)?;
                let embedding_bytes: Vec<u8> = r.get(4)?;

                pages.push(FoundPage {
                    distance,
                    url,
                    title,
                    text,
                    embedding: unsafe { bytes_to_embedding(&embedding_bytes.try_into().unwrap())? }
                        .to_vec(),
                });
            } else {
                println!("Page not found in DB: {}", id);
            }
        }
        println!("Search completed in {} us", duration.as_micros(),);

        Ok(SearchResult {
            pages,
            pages_searched: self.index.size(),
        })
    }

    pub fn insert(&mut self, page: ExtractedPage) -> Result<(), anyhow::Error> {
        let mut find_by_url = self
            .sqlite
            .prepare("SELECT id FROM page WHERE url = ?1")
            .unwrap();
        let found = find_by_url.query_row(&[&page.url], |row| row.get::<_, u64>(0));
        if found.is_ok() {
            // Already exists!
            println!("Already have with id {}", page.url);
            return Ok(());
        }

        let q = &self.model.encode(&[page.combined])?[0];

        if !is_normalized(q.as_slice().try_into()?) {
            bail!("Insert embedding is not normalized");
        }

        // Insert into DB
        let embedding: &[u8; EM_LEN * 4] = unsafe { vector_embedding_to_bytes(q)? };
        self.sqlite.execute(
            "INSERT INTO page (url, title, text, embedding) VALUES (?1, ?2, ?3, ?4)",
            (page.url, page.title, page.text, embedding),
        )?;
        let id: u64 = self
            .sqlite
            .query_row("SELECT last_insert_rowid()", (), |row| row.get(0))?;

        // Insert into index
        if self.index.size() == self.index.capacity() {
            // Weirdly enough we have to reserve capacity ourselves.
            self.index.reserve(self.index.size() + 1024)?;
        }
        self.index.add(id, q)?;
        Ok(())
    }

    /** Check if all our data is OK */
    pub fn verify(&mut self) -> anyhow::Result<()> {
        println!(
            "[Search Provider] Checking vector sizes... {}",
            self.page_count()?
        );

        let progress = default_progress_bar(self.page_count()?);
        let mut s = self.sqlite.prepare("SELECT id, embedding FROM page")?;
        let mut qq = s.query(())?;

        let mut wrong_length = 0;
        let mut not_normalized = 0;
        while let Some(r) = qq.next()? {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            progress.inc(1);
            let id: u64 = r.get(0)?;
            let embedding: Vec<u8> = r.get(1)?;
            if embedding.len() != EM_LEN * 4 {
                wrong_length += 1;
                continue;
            }
            let q: &[f32] = unsafe { bytes_to_embedding(embedding.as_slice().try_into()?)? };
            let embedding = q.try_into()?;
            if !is_normalized(embedding) {
                not_normalized += 1;
            }
        }
        progress.finish();
        if wrong_length > 0 || not_normalized > 0 {
            bail!(
                "Wrong number of bytes: {} Not normalized: {}",
                wrong_length,
                not_normalized
            );
        }
        Ok(())
    }
}
