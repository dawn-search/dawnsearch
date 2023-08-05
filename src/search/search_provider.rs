/*
   Copyright 2023 Krol Inventions B.V.

   This file is part of DawnSearch.

   DawnSearch is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   DawnSearch is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with DawnSearch.  If not, see <https://www.gnu.org/licenses/>.
*/

use crate::search::page_source::ExtractedPage;
use crate::search::vector::{bytes_to_embedding, is_normalized, vector_embedding_to_bytes, EM_LEN};
use crate::util::default_progress_bar;
use anyhow::anyhow;
use anyhow::bail;
use cxx::UniquePtr;
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType,
};
use std::iter::zip;
use std::path::Path;
use std::time::Instant;
use std::{self, fs};
use std::{str, usize};
use tokio_util::sync::CancellationToken;
use usearch::ffi::{new_index, Index, IndexOptions, MetricKind, ScalarKind};

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
    pub id: usize,
    pub distance: f32,
    pub url: String,
    pub title: String,
    pub text: String,
}

pub struct SearchProvider {
    model: SentenceEmbeddingsModel,
    index: UniquePtr<Index>,

    sqlite: rusqlite::Connection,

    shutdown_token: CancellationToken,
    data_dir: String,
}

impl SearchProvider {
    pub fn new(
        data_dir: String,
        shutdown_token: CancellationToken,
    ) -> Result<SearchProvider, anyhow::Error> {
        let start = Instant::now();
        let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
            .with_device(tch::Device::Cpu)
            .create_model()?;

        let duration = start.elapsed();
        println!(
            "[Search Provider]  Loaded model in {} ms",
            duration.as_millis()
        );

        // Database
        let sqlite = rusqlite::Connection::open(Path::new(&data_dir).join("dawnsearch.sqlite"))?;

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
        let index = new_index(&INDEX_OPTIONS)?;

        let mut search_provider = SearchProvider {
            model,
            index,
            sqlite,
            shutdown_token: shutdown_token.clone(),
            data_dir: data_dir.clone(),
        };

        let index_path_path = Path::new(&data_dir).join("index.usearch");
        let index_path = index_path_path
            .to_str()
            .ok_or(anyhow!("Could not convert path to string"))?;
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

    pub fn local_space_available(&mut self) -> bool {
        self.page_count().unwrap() < 1000000 // TODO: move to config
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        self.save()?;
        Ok(())
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        let index_path_path = Path::new(&self.data_dir).join("index.usearch");
        let path = index_path_path
            .to_str()
            .ok_or(anyhow!("Could not convert index path"))?;
        self.index.save(path)?;
        println!("[Search Provider] Saved index to {}", path);
        Ok(())
    }

    pub fn get_embedding(&self, query: &str) -> anyhow::Result<Vec<f32>> {
        Ok(self.model.encode(&[query])?[0].clone())
    }

    pub fn search(&self, query: &str) -> Result<SearchResult, anyhow::Error> {
        let q = &self.model.encode(&[query])?[0];
        self.search_embedding(q)
    }

    pub fn search_like(&self, id: usize) -> Result<SearchResult, anyhow::Error> {
        let mut s = self
            .sqlite
            .prepare("SELECT embedding FROM page WHERE id  = ?1")?;
        let mut qq = s.query(&[&id])?;
        if let Some(r) = qq.next()? {
            let embedding_bytes: Vec<u8> = r.get(0)?;
            let embedding = unsafe { bytes_to_embedding(embedding_bytes.as_slice().try_into()?)? };

            return self.search_embedding(&embedding.to_vec());
        }
        bail!("Page not found in DB: {}", id);
    }

    pub fn search_embedding(
        &self,
        query_embedding: &Vec<f32>,
    ) -> Result<SearchResult, anyhow::Error> {
        if !is_normalized(query_embedding.as_slice().try_into()?) {
            bail!("Search vector is not normalized");
        }
        let mut pages = Vec::new();

        let start = Instant::now();

        // Read back the tags
        let results = self.index.search(query_embedding, 20)?;

        let duration = start.elapsed();

        let mut s = self
            .sqlite
            .prepare("SELECT id, url, title, text FROM page WHERE id  = ?1")?;
        for (distance, id) in zip(results.distances, results.labels) {
            let mut qq = s.query(&[&id])?;
            if let Some(r) = qq.next()? {
                let id: u64 = r.get(0)?;
                let url = r.get(1)?;
                let title = r.get(2)?;
                let text: String = r.get(3)?;

                pages.push(FoundPage {
                    id: id as usize,
                    distance,
                    url,
                    title,
                    text,
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
        if !self.local_space_available() {
            bail!("No space available");
        }
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
            let _id: u64 = r.get(0)?;
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
