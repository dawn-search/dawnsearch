use std::io::{self, BufRead};
use std::time::Instant;
use std::{self};
use std::{str, usize};

use arecibo::best_results::{BestResults, NodeReference};
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};
use std::env;

use arecibo::document_embeddings::DocumentEmbeddings;
use arecibo::vector::{Distance, Embedding, ToI16, EM_LEN};

const BUCKET_COUNT: usize = 200;
const INSERT_COUNT: usize = 3;
const SEARCH_COUNT: usize = 10;

#[derive(Clone)]
struct Entry {
    address: Embedding<i16>,
    id: usize,
}

struct Node {
    center: Embedding<i16>,
    entries: Vec<Entry>,
}

impl Node {
    fn search(&self, address: &Embedding<i16>, best: &mut BestResults<u64>) {
        for entry in &self.entries {
            best.insert(NodeReference::<u64> {
                id: entry.id,
                distance: address.distance_ip(&entry.address),
            });
        }
    }

    fn insert(&mut self, entry: Entry) {
        self.entries.push(entry);
    }
}

struct BucketSearch {
    nodes: Vec<Node>,
}

impl BucketSearch {
    fn new() -> BucketSearch {
        BucketSearch { nodes: Vec::new() }
    }

    fn add_bucket(&mut self, address: &Embedding<i16>) {
        self.nodes.push(Node {
            center: address.clone(),
            entries: Vec::new(),
        });
    }

    fn insert(&mut self, entry: Entry) {
        let mut best = self.find_nodes(&entry.address, INSERT_COUNT);
        best.sort();
        for nn in best.results() {
            let node = &mut self.nodes[nn.id];
            node.insert(entry.clone());
        }
    }

    fn search(&self, address: &Embedding<i16>, mut results: &mut BestResults<u64>) {
        let mut nodes = self.find_nodes(address, SEARCH_COUNT);
        nodes.sort();

        for n in nodes.results() {
            let node = &self.nodes[n.id];
            node.search(address, &mut results)
        }
    }

    fn find_nodes(&self, address: &Embedding<i16>, count: usize) -> BestResults<u64> {
        let mut best = BestResults::new(count);
        for (node_id, node) in self.nodes.iter().enumerate() {
            best.insert(NodeReference::<u64> {
                id: node_id,
                distance: address.distance_ip(&node.center),
            });
        }
        best
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let warc_dir = &args[1];

    let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
        .with_device(tch::Device::Cpu)
        .create_model()?;

    let document_embeddings = DocumentEmbeddings::load(&warc_dir)?;

    let mut bucket_search = BucketSearch::new();

    let mut count = 0;
    for page in 0..document_embeddings.files() {
        for entry in 0..document_embeddings.entries(page) {
            let p = document_embeddings.entry(page, entry);
            bucket_search.add_bucket(&p.vector.to_i16());
            count += 1;
            if count > BUCKET_COUNT {
                break;
            }
        }
        if count > BUCKET_COUNT {
            break;
        }
    }

    let mut searched_pages_count = 0;
    for page in 0..document_embeddings.files() {
        for entry in 0..document_embeddings.entries(page) {
            let p = document_embeddings.entry(page, entry);
            bucket_search.insert(Entry {
                address: p.vector.to_i16(),
                id: searched_pages_count,
            });
            searched_pages_count += 1;
        }
    }

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
        let query_embedding: &[f32; EM_LEN] = q.as_slice().try_into().unwrap();

        let start = Instant::now();

        let mut results = BestResults::new(10);
        bucket_search.search(&query_embedding.to_i16(), &mut results);
        results.sort();

        let duration = start.elapsed();

        let mut count = 0;
        for result in results.results() {
            count += 1;
            if count > 10 {
                break;
            }
            let (file, entry) = document_embeddings.linear_to_segmented(result.id);
            // let e = document_embeddings.entry(file, entry);
            let url: &[u8] = document_embeddings.url(file, entry);
            let title: &[u8] = document_embeddings.title(file, entry);
            let df = result.distance as f32 / (i16::MAX as f32).powf(2.0);
            println!(
                "{:.2}: {} - {}",
                df,
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
