use std::collections::HashSet;
use std::io::{self, BufRead};
use std::str;
use std::time::Instant;
use std::{self, cmp};

use rand::Rng;
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};
use std::env;

use arecibo::best_results::{BestResults, NodeReference};
use arecibo::document_embeddings::DocumentEmbeddings;
use arecibo::vector::{random_address, Distance, EM_LEN};

use arecibo::vector::Embedding;

struct NswNode {
    address: Embedding<f32>,
    peers: Vec<NodeReference<f32>>,
}

struct Nsw {
    nodes: Vec<NswNode>,
}

const DEBUG_SEARCH: bool = false;
const STRUCTURE_NODES: usize = 10;

impl Nsw {
    fn new() -> Nsw {
        Nsw { nodes: Vec::new() }
    }

    fn insert(&mut self, address: &Embedding<f32>) {
        let mut rng = rand::thread_rng();
        let m = cmp::min(STRUCTURE_NODES, self.nodes.len());
        let node_id = if m > 0 { rng.gen_range(0..m) } else { 0 };

        let results = self.search(&address, 16, node_id);
        // Insert links from new node.

        let mut peers: Vec<NodeReference<f32>> = Vec::new();
        for r in results.results() {
            if !peers.iter().any(|x| x.id == r.id) {
                peers.push(r.clone());
            }
        }
        let sort = true; // The sort really helps with getting to the right spot.
        if sort {
            peers.sort_by(|a, b| b.distance.partial_cmp(&a.distance).unwrap());
        }
        let node = NswNode {
            address: address.clone(),
            peers,
        };

        let node_id = self.nodes.len();

        // Insert links to new node.
        for other in &node.peers {
            self.nodes[other.id].peers.push(NodeReference {
                id: node_id,
                distance: other.distance,
            });
            if sort {
                self.nodes[other.id]
                    .peers
                    .sort_by(|a, b| b.distance.partial_cmp(&a.distance).unwrap());
            }
        }

        self.nodes.push(node);
    }

    fn expand(&self, address: &Embedding<f32>, mut results: &mut BestResults<f32>) {
        let mut seen = HashSet::new();
        results.sort();
        let targets = results.results().clone();
        for t in targets {
            self.expand_inner(address, t.id, &mut seen, &mut results);
            // break;
        }
    }

    fn expand_inner(
        &self,
        address: &Embedding<f32>,
        node_id: usize,
        seen: &mut HashSet<usize>,
        results: &mut BestResults<f32>,
    ) {
        if seen.contains(&node_id) {
            return;
        }
        seen.insert(node_id);
        let node = &self.nodes[node_id];
        let dist = Distance::distance(&node.address, &address);
        if dist >= results.worst_distance() {
            return;
        }
        results.insert(NodeReference {
            id: node_id,
            distance: dist,
        });
        for x in &node.peers {
            self.expand_inner(address, x.id, seen, results);
        }
    }

    fn search(&mut self, address: &Embedding<f32>, count: usize, start: usize) -> BestResults<f32> {
        let mut results = self.search2(address, count, start);
        if results.len() == 0 {
            return results;
        }
        self.expand(address, &mut results);
        results
    }

    fn search2(
        &mut self,
        address: &Embedding<f32>,
        count: usize,
        start: usize,
    ) -> BestResults<f32> {
        if self.nodes.len() == 0 {
            return BestResults::new(0);
        }
        let mut node_id = start;
        let mut node_score = address.distance(&self.nodes[node_id].address);

        let mut results = BestResults::new(count);
        results.insert(NodeReference {
            id: node_id,
            distance: node_score,
        });

        if DEBUG_SEARCH {
            println!(
                "Search starts at node {} with score {}",
                node_id, node_score
            );
        }

        loop {
            if self.nodes[node_id].peers.len() == 0 {
                break; // Can happen for our first node.
            }
            let mut best_next_peer_id = None;
            let mut best_next_peer_score = node_score;
            for (_peer_index, peer_ref) in self.nodes[node_id].peers.iter().enumerate() {
                let peer = &self.nodes[peer_ref.id];
                let score = address.distance(&peer.address);

                results.insert(NodeReference {
                    id: peer_ref.id,
                    distance: score,
                });
                if DEBUG_SEARCH {
                    println!("Have {} results ", results.len());
                }

                // Find next peer to move into.
                if score < best_next_peer_score {
                    best_next_peer_id = Some(peer_ref);
                    best_next_peer_score = score;
                }
            }
            if let Some(s) = best_next_peer_id {
                node_id = s.id;
                node_score = best_next_peer_score;
                if DEBUG_SEARCH {
                    println!("Continuing with node {} with score {}", node_id, node_score);
                }
            } else {
                // We're done.
                if DEBUG_SEARCH {
                    println!(
                        "Search completed with {} entries, worst score {}",
                        results.len(),
                        results.worst_distance()
                    );
                }
                break;
            }
        }
        results
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let warc_dir = &args[1];

    let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
        .with_device(tch::Device::Cpu)
        .create_model()?;

    let document_embeddings = DocumentEmbeddings::load(&warc_dir)?;

    let mut searched_pages_count = 0;

    let start = Instant::now();

    let mut nsw = Nsw::new();

    for _i in 0..STRUCTURE_NODES {
        nsw.insert(&random_address());
    }
    for file in 0..document_embeddings.files() {
        eprint!("File {}", file);
        for entry in 0..document_embeddings.entries(file) {
            let p = document_embeddings.entry(file, entry);
            nsw.insert(&p.vector);
            searched_pages_count += 1;
            if searched_pages_count % 1000 == 0 {
                eprint!(".")
            }
        }
        println!("");
        // break;
    }

    let duration = start.elapsed();
    println!("");
    println!("Generated index in {:.1} ms", duration.as_millis());

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

        let mut rng = rand::thread_rng();
        let node_id = rng.gen_range(0..STRUCTURE_NODES);

        let mut results = nsw.search(&query_embedding, 20, node_id);
        results.sort();

        let mut count = 0;
        for result in results.results() {
            count += 1;
            if count > 10 {
                break;
            }
            if result.id < STRUCTURE_NODES {
                println!("*** {}", result.id);
                continue;
            }
            let (file, entry) =
                document_embeddings.linear_to_segmented(result.id - STRUCTURE_NODES);
            let _e = document_embeddings.entry(file, entry);
            let url: &[u8] = document_embeddings.url(file, entry);
            let title: &[u8] = document_embeddings.title(file, entry);
            println!(
                "{}: {} - {}",
                result.distance,
                unsafe { str::from_utf8_unchecked(title) },
                unsafe { str::from_utf8_unchecked(url) }
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
