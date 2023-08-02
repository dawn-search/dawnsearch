use std::iter::zip;
use std::mem::transmute;
use std::time::Instant;
use std::{self, fs};
use std::{str, usize};

use cxx::UniquePtr;
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType,
};
use tokio_util::sync::CancellationToken;
use usearch::ffi::{new_index, Index, IndexOptions, MetricKind, ScalarKind};

use crate::page_source::ExtractedPage;
use crate::util::default_progress_bar;
use crate::vector::{Embedding, EM_LEN};

const INDEX_OPTIONS: IndexOptions = IndexOptions {
    dimensions: EM_LEN,
    metric: MetricKind::IP,
    quantization: ScalarKind::F8,
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

        Ok(search_provider)
    }

    fn fill_index_from_db(&mut self) -> Result<(), anyhow::Error> {
        // Fill from DB
        let count = self
            .sqlite
            .query_row("SELECT count(*) FROM page", (), |row| {
                row.get::<_, usize>(0)
            })?;
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
            let q: &[f32] = unsafe { transmute(embedding.as_slice()) };

            self.index.add(id, q).unwrap();
        }
        progress.finish_and_clear();
        Ok(())
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        println!("[Search Provider] Shutting down...");
        self.index.save("index.usearch")?;
        println!("[Search Provider] Shutdown completed");
        Ok(())
    }
    // pub fn load(warc_dir: &str) -> Result<SearchProvider, anyhow::Error> {
    //     let start = Instant::now();

    //     print!("Loading model...");

    //     let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
    //         .with_device(tch::Device::Cpu)
    //         .create_model()?;

    //     let duration = start.elapsed();
    //     println!(" {} ms", duration.as_millis());

    //     let mut data = Vec::new();
    //     let document_embeddings = DocumentEmbeddings::load(&warc_dir)?;
    //     for page in 0..document_embeddings.files() {
    //         for entry in 0..document_embeddings.entries(page) {
    //             let url = str::from_utf8(document_embeddings.url(page, entry))?.to_string();
    //             let title = str::from_utf8(document_embeddings.title(page, entry))?.to_string();
    //             let text = String::new();

    //             data.push(PageData { url, title, text })
    //         }
    //     }

    //     let total_documents = document_embeddings.len();

    //     let index = new_index(&INDEX_OPTIONS).unwrap();

    //     let index_path = "index.usearch";
    //     if !fs::metadata(index_path).is_ok() || !index.view(index_path).is_ok() {
    //         println!("Recalculating index...");
    //         index.reserve(document_embeddings.len())?;

    //         let progress = default_progress_bar(36000);
    //         );
    //         let mut searched_pages_count = 0;
    //         for page in 0..document_embeddings.files() {
    //             for entry in 0..document_embeddings.entries(page) {
    //                 progress.set_position(searched_pages_count);
    //                 let p = document_embeddings.entry(page, entry);
    //                 index.add(searched_pages_count, &p.vector)?;
    //                 searched_pages_count += 1;
    //             }
    //         }
    //         progress.finish();
    //         index.save(index_path)?;
    //     } else {
    //         println!("Loaded index {}", index_path);
    //     }
    //     Ok(SearchProvider {
    //         model,
    //         // document_embeddings,
    //         index,
    //         data,
    //     })
    // }

    pub fn search(&self, query: &str) -> Result<SearchResult, anyhow::Error> {
        let mut pages = Vec::new();

        let q = &self.model.encode(&[query]).unwrap()[0];
        let query_embedding: &Embedding<f32> = q.as_slice().try_into().unwrap();

        let start = Instant::now();

        // Read back the tags
        let results = self.index.search(query_embedding, 20).unwrap();

        let duration = start.elapsed();

        let mut s = self
            .sqlite
            .prepare("SELECT id, url, title, text FROM page WHERE id  = ?1")
            .unwrap();
        for (distance, id) in zip(results.distances, results.labels) {
            let mut qq = s.query(&[&id]).unwrap();
            let r = qq.next().unwrap().unwrap();
            let _id: u64 = r.get(0).unwrap();
            let url = r.get(1).unwrap();
            let title = r.get(2).unwrap();
            let _text: String = r.get(3).unwrap();

            pages.push(FoundPage {
                distance,
                url,
                title,
            });
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

        println!("Inserting {}", page.url);

        let q = &self.model.encode(&[page.combined]).unwrap()[0];

        // Insert into DB
        let embedding: &[u8] = unsafe { transmute(q.as_slice()) };
        self.sqlite
            .execute(
                "INSERT INTO page (url, title, text, embedding) VALUES (?1, ?2, ?3, ?4)",
                (page.url, page.title, page.text, embedding),
            )
            .unwrap();
        let id: u64 = self
            .sqlite
            .query_row("SELECT last_insert_rowid()", (), |row| row.get(0))
            .unwrap();

        // Insert into index
        if self.index.size() == self.index.capacity() {
            // Weirdly enough we have to reserve capacity ourselves.
            self.index.reserve(self.index.size() + 1024)?;
        }
        self.index.add(id, q)?;
        Ok(())
    }
}
