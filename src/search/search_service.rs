use crate::messages::SearchProviderMessage;
use crate::messages::SearchProviderMessage::*;
use crate::search_provider::SearchProvider;
use crate::search_provider::SearchResult;
use std::sync::mpsc::Receiver;
use tokio_util::sync::CancellationToken;

pub struct SearchService {
    pub data_dir: String,
    pub shutdown_token: CancellationToken,
    pub search_provider_receiver: Receiver<SearchProviderMessage>,
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
                    let result = match search_provider.search(&query) {
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
