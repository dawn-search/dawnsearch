use std::io::{self, BufRead};
use std::time::Instant;
use std::{self};
use std::{result, str};

use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};
use std::env;

use arecibo::document_embeddings::DocumentEmbeddings;
use arecibo::vector::{distance, distance_upper_bound, EM_LEN};

type Address = [f32; EM_LEN];

struct NswNode {
    address: Address,
    peers: Vec<usize>,
}

struct Nsw {
    nodes: Vec<NswNode>,
    results: Vec<SearchResult>,
}

#[derive(Clone)]
struct SearchResult {
    id: usize,
    score: f32,
}

const DEBUG_SEARCH: bool = false;

impl Nsw {
    fn new() -> Nsw {
        Nsw {
            nodes: Vec::new(),
            results: Vec::new(),
        }
    }

    fn insert(&mut self, address: &Address) {
        self.search(&address, 30);
        // Insert links from new node.

        let node = NswNode {
            address: address.clone(),
            peers: self.results.iter().map(|x| x.id).collect(),
        };

        let node_id = self.nodes.len();

        // Insert links to new node.
        for other in &node.peers {
            self.nodes[*other].peers.push(node_id);
        }

        self.nodes.push(node);
    }

    fn search(&mut self, address: &Address, count: usize) {
        if self.nodes.len() == 0 {
            self.results.clear();
            return;
        }
        let mut worst_result_index = 0;
        let mut worst_score = distance(&self.nodes[0].address, &address);

        self.results.fill(SearchResult {
            id: 0,
            score: worst_score,
        });
        self.results.resize(
            count,
            SearchResult {
                id: 0,
                score: worst_score,
            },
        );

        let mut node_id = 0;
        let mut node_score = worst_score;

        if DEBUG_SEARCH {
            println!(
                "Search starts at node {} with score {}",
                node_id, node_score
            );
        }

        loop {
            if self.nodes[node_id].peers.len() == 0 {
                break; // Should not happen, but you never know.
            }
            let mut best_next_peer_id = None;
            let mut best_next_peer_score = node_score;
            for (peer_index, peer_id) in self.nodes[node_id].peers.iter().enumerate() {
                if *peer_id == node_id {
                    continue; // Don't follow the link back.
                }
                if self.results.iter().any(|x| x.id == *peer_id) {
                    continue; // We already have this one.
                }

                let peer = &self.nodes[*peer_id];
                let score = distance(&peer.address, &address);

                // Update our result list.
                if self.results.len() < count {
                    // Just add, keeping track of worst_score.
                    if score > worst_score {
                        worst_score = score;
                        worst_result_index = self.results.len();
                    }
                    self.results.push(SearchResult {
                        id: *peer_id,
                        score,
                    });
                } else {
                    if score < worst_score {
                        // Put in list of results.
                        worst_score = score;
                        self.results[worst_result_index] = SearchResult {
                            id: *peer_id,
                            score,
                        };
                        // Recalculate worst, as the newly placed one may quite good.
                        for (i, result) in self.results.iter().enumerate() {
                            if result.score > worst_score {
                                worst_score = result.score;
                                worst_result_index = i;
                            }
                        }
                    }
                }

                // Find next peer to move into.
                if score < best_next_peer_score {
                    best_next_peer_id = Some(peer_id);
                    best_next_peer_score = score;

                    // Eager
                    break;
                }
            }
            if let Some(s) = best_next_peer_id {
                node_id = *s;
                node_score = best_next_peer_score;
                if DEBUG_SEARCH {
                    println!("Continuing with node {} with score {}", node_id, node_score);
                }
            } else {
                // We're done.
                if DEBUG_SEARCH {
                    println!(
                        "Search completed with {} entries, worst score {}",
                        self.results.len(),
                        worst_score
                    );
                }
                break;
            }
        }
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

    let mut nsw = Nsw::new();
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
        //        break;
    }

    let stdin = io::stdin();
    eprint!("> ");
    for q in stdin.lock().lines() {
        println!("");
        let query = q.unwrap();

        let q = &model.encode(&[query]).unwrap()[0];
        let query_embedding: &[f32; EM_LEN] = q.as_slice().try_into().unwrap();

        let start = Instant::now();

        nsw.search(&query_embedding, 10);
        nsw.results
            .sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
        for result in &nsw.results {
            let (file, entry) = document_embeddings.linear_to_segmented(result.id);
            let url: &[u8] = document_embeddings.url(file, entry);
            let title: &[u8] = document_embeddings.title(file, entry);
            println!(
                "{}: {} - {}",
                result.score,
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
