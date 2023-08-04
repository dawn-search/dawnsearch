use crate::net::udp_service::UdpM;
use crate::search::messages::SearchProviderMessage;
use crate::search::messages::SearchProviderMessage::*;
use crate::search::search_provider::FoundPage;
use crate::search::search_provider::SearchProvider;
use crate::search::search_provider::SearchResult;
use std::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

pub struct SearchService {
    pub data_dir: String,
    pub shutdown_token: CancellationToken,
    pub search_provider_receiver: Receiver<SearchProviderMessage>,
    pub udp_tx: tokio::sync::mpsc::Sender<UdpM>,
}

impl SearchService {
    pub fn start(&mut self) {
        let mut search_provider =
            match SearchProvider::new(self.data_dir.clone(), self.shutdown_token.clone()) {
                Err(e) => {
                    println!("Failed to load search provider {}", e);
                    println!("{}", e.backtrace());
                    return;
                }
                Ok(s) => s,
            };
        println!("SearchProvider ready");
        while let Ok(message) = self.search_provider_receiver.recv() {
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
                            }
                        }
                    };

                    let udp_tx2 = self.udp_tx.clone();
                    println!("Spawning background search");
                    tokio::spawn(async move {
                        println!("Background search running");
                        // Also fire it off to the network.
                        let (otxx, orxx) = oneshot::channel();
                        udp_tx2
                            .send(UdpM::Search {
                                embedding,
                                tx: otxx,
                            })
                            .await
                            .unwrap();
                        println!("Waiting for results");
                        let r = orxx.await.unwrap();
                        println!("Results: {:?}", r);

                        let mut r2 = result.pages;

                        // Add our own results to this.
                        for x in r {
                            r2.push(FoundPage {
                                id: 0,
                                title: x.title,
                                distance: x.distance,
                                url: x.url,
                                text: x.text,
                            });
                        }

                        println!("[Search Service] Sending results back");
                        otx.send(SearchResult {
                            pages: r2,
                            pages_searched: 0,
                        })
                        .expect("Send response");
                        println!("[Search Service] Done");
                    });
                }
                EmbeddingSearch { otx, embedding } => {
                    let result = match search_provider.search_embedding(&embedding) {
                        Ok(r) => r,
                        Err(e) => {
                            println!("Failed to perform query: {}", e);
                            SearchResult {
                                pages: Vec::new(),
                                pages_searched: 0,
                            }
                        }
                    };
                    otx.send(result).expect("Send response");
                }
                MoreLikeSearch { otx, id } => {
                    let result = match search_provider.search_like(id) {
                        Ok(r) => r,
                        Err(e) => {
                            println!("Failed to perform query: {}", e);
                            SearchResult {
                                pages: Vec::new(),
                                pages_searched: 0,
                            }
                        }
                    };
                    otx.send(result).expect("Send response");
                }
                ExtractedPageMessage { page } => match search_provider.insert(page) {
                    Err(e) => println!("Failed to insert {}", e),
                    _ => {}
                },
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
}
