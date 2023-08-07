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

use crate::config::Config;
use crate::net::udp_service::UdpMsg;
use crate::search::best_results::BestResults;
use crate::search::best_results::NodeReference;
use crate::search::search_msg::SearchMsg;
use crate::search::search_msg::SearchMsg::*;
use crate::search::search_provider::FoundPage;
use crate::search::search_provider::SearchProvider;
use crate::search::search_provider::SearchResult;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

pub struct SearchService {
    pub config: Config,
    pub shutdown_token: CancellationToken,
    pub search_rx: Receiver<SearchMsg>,
    pub udp_tx: tokio::sync::mpsc::Sender<UdpMsg>,
    pub search_tx: SyncSender<SearchMsg>,
}

impl SearchService {
    pub fn start(&mut self) {
        let mut search_provider =
            match SearchProvider::new(self.config.data_dir.clone(), self.shutdown_token.clone()) {
                Err(e) => {
                    println!("Failed to load search provider {}", e);
                    return;
                }
                Ok(s) => s,
            };
        println!("[Search] ready");
        while let Ok(message) = self.search_rx.recv() {
            if self.config.debug > 0 {
                println!("[Search] Received message {:?}", message);
            }
            match message {
                TextSearch { otx, query } => {
                    let embedding = search_provider.get_embedding(&query).unwrap();
                    let result = match search_provider.search_embedding(&embedding) {
                        Ok(r) => r,
                        Err(e) => {
                            println!("Failed to perform query: {}", e);
                            SearchResult {
                                pages: Vec::new(),
                                pages_searched: 0,
                                servers_contacted: 0,
                            }
                        }
                    };
                    self.search_remote(result, embedding, otx);
                }
                EmbeddingSearch {
                    otx,
                    embedding,
                    search_remote,
                } => {
                    let result = match search_provider.search_embedding(&embedding) {
                        Ok(r) => r,
                        Err(e) => {
                            println!("Failed to perform query: {}", e);
                            SearchResult {
                                pages: Vec::new(),
                                pages_searched: 0,
                                servers_contacted: 0,
                            }
                        }
                    };
                    if search_remote {
                        self.search_remote(result, embedding, otx);
                    } else {
                        otx.send(result).expect("Sending embedding search result");
                    }
                }
                MoreLikeSearch {
                    otx,
                    instance_id,
                    page_id,
                } => {
                    if instance_id == "" {
                        if let Ok(embedding) = search_provider.embedding_for_page(page_id) {
                            let result = match search_provider.search_embedding(&embedding) {
                                Ok(r) => r,
                                Err(e) => {
                                    println!("Failed to perform query: {}", e);
                                    SearchResult {
                                        pages: Vec::new(),
                                        pages_searched: 0,
                                        servers_contacted: 0,
                                    }
                                }
                            };
                            self.search_remote(result, embedding, otx);
                        }
                    } else {
                        // Reference to a peer, ask it for the embedding so we can search for it.
                        let (otxx, orxx) = oneshot::channel();
                        let search_tx2 = self.search_tx.clone();
                        let udp_tx2 = self.udp_tx.clone();
                        let debug = self.config.debug;
                        tokio::spawn(async move {
                            udp_tx2
                                .send(UdpMsg::GetEmbedding {
                                    instance_id,
                                    page_id,
                                    tx: otxx,
                                })
                                .await
                                .unwrap();
                            let embedding = orxx.await.unwrap();
                            if debug > 0 {
                                println!(
                                    "[Search] Announce: got a vector of length {} back from UDP",
                                    embedding.len()
                                );
                            }
                            // Pass it back into ourselves as a normal query.
                            search_tx2
                                .send(SearchMsg::EmbeddingSearch {
                                    otx,
                                    embedding,
                                    search_remote: true,
                                })
                                .unwrap();
                        });
                    }
                }
                ExtractedPage { page, from_network } => {
                    if search_provider.local_space_available() {
                        if let Err(e) = search_provider.insert(page.clone()) {
                            eprintln!("Failed to insert {}", e);
                        }
                    }
                    if !from_network {
                        // Insert on the network.
                        let tx = self.udp_tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) = tx.send(UdpMsg::Insert { page }).await {
                                eprintln!("Error occurred sending to the UDP system: {}", e)
                            }
                        });
                    }
                }
                Stats { otx } => {
                    let stats = search_provider.stats();
                    otx.send(stats).expect("Send response");
                }
                GetEmbedding { page_id, otx } => {
                    let em = search_provider.embedding_for_page(page_id).unwrap();
                    otx.send(em).expect("Send response");
                }
                Save => {
                    search_provider.save().unwrap();
                }
                Shutdown => {
                    search_provider.shutdown().unwrap();
                    break;
                }
            }
        }
    }

    fn search_remote(
        &mut self,
        result: SearchResult,
        embedding: Vec<f32>,
        otx: oneshot::Sender<SearchResult>,
    ) {
        let mut all_found_pages = result.pages;

        if self.config.debug > 0 {
            println!("[Search] got {} local results", all_found_pages.len());
        }

        // Store them in a BestResults
        let mut best = BestResults::new(20);
        for (id, page) in all_found_pages.iter().enumerate() {
            best.insert(NodeReference {
                id,
                distance: page.distance,
            });
        }
        // We now also know what our worst result is.
        let worst_distance = best.worst_distance();

        let udp_tx2 = self.udp_tx.clone();
        let debug = self.config.debug;
        tokio::spawn(async move {
            // Also fire it off to the network.
            let (otxx, orxx) = oneshot::channel();
            udp_tx2
                .send(UdpMsg::Search {
                    embedding,
                    distance_limit: Some(worst_distance),
                    tx: otxx,
                })
                .await
                .unwrap();
            let r = orxx.await.unwrap();
            if debug > 0 {
                println!(
                    "[Search] remote: got {} search results from UDP",
                    r.results.len()
                );
            }

            // Add our own results to this.
            let total_pages = result.pages_searched;
            for x in r.results {
                best.insert(NodeReference {
                    id: all_found_pages.len(),
                    distance: x.distance,
                });
                all_found_pages.push(FoundPage {
                    instance_id: x.instance_id,
                    page_id: x.page_id,
                    title: x.title,
                    distance: x.distance,
                    url: x.url,
                    text: x.text,
                });
            }

            // We have collected our results.
            best.sort();
            let real_results: Vec<FoundPage> = best
                .results()
                .iter()
                .map(|nr| all_found_pages[nr.id].clone())
                .collect();

            otx.send(SearchResult {
                pages: real_results,
                pages_searched: total_pages + r.pages_searched,
                servers_contacted: r.servers_contacted,
            })
            .expect("Send response");
        });
    }
}
