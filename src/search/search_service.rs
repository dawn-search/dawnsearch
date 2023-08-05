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
                    tokio::spawn(async move {
                        // Also fire it off to the network.
                        let (otxx, orxx) = oneshot::channel();
                        udp_tx2
                            .send(UdpM::Search {
                                embedding,
                                tx: otxx,
                            })
                            .await
                            .unwrap();
                        let r = orxx.await.unwrap();

                        let mut r2 = result.pages;

                        // Add our own results to this.
                        let total_pages = result.pages_searched;
                        for x in r {
                            r2.push(FoundPage {
                                id: 0,
                                title: x.title,
                                distance: x.distance,
                                url: x.url,
                                text: x.text,
                            });
                            // TODO: add pages searched from the network.
                        }

                        otx.send(SearchResult {
                            pages: r2,
                            pages_searched: total_pages,
                        })
                        .expect("Send response");
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
                ExtractedPageMessage { page, from_network } => {
                    if search_provider.local_space_available() {
                        match search_provider.insert(page.clone()) {
                            Err(e) => println!("Failed to insert {}", e),
                            _ => {}
                        }
                    }
                    if !from_network {
                        // Insert on the network.
                        let tx = self.udp_tx.clone();
                        tokio::spawn(async move {
                            tx.send(UdpM::Insert { page }).await.unwrap();
                        });
                    }
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
}
