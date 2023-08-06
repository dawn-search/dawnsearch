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
use crate::net::udp_messages::{PeerInfo, UdpMessage};
use crate::search::messages::SearchProviderMessage;
use crate::search::page_source::ExtractedPage;
use crate::search::vector::ToFrom24;
use crate::util::{now, slice_up_to};
use anyhow::bail;
use rand::distributions::Alphanumeric;
use rand::seq::SliceRandom;
use rand::Rng;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc::SyncSender;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::oneshot;

#[cfg(feature = "upnp")]
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
#[cfg(feature = "upnp")]
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

pub const TRACKER_UDP_PORT: u32 = 7230;
const UDP_PORT: u32 = 7231; // Looks like nobody is using this one yet.

pub async fn find_port() -> anyhow::Result<UdpSocket> {
    let mut port_inc = 0;

    let socket = loop {
        let port = UDP_PORT + port_inc;
        let addr = format!("0.0.0.0:{}", port);

        match UdpSocket::bind(&addr).await {
            Ok(s) => {
                break s;
            }
            Err(e) => {
                println!("Port in use? {}", e);
            }
        }

        port_inc += 1;
        if port_inc >= 10 {
            bail!("No free port found for UDP");
        }
    };
    Ok(socket)
}

#[derive(Debug, Clone)]
pub struct PageFromNetwork {
    pub instance_id: String,
    pub page_id: usize,
    pub distance: f32,
    pub url: String,
    pub title: String,
    pub text: String,
}

pub struct ActiveSearch {
    search_id: u64,
    deadline: Instant,
    /** Channel to which we send the results. */
    tx: oneshot::Sender<NetworkSearchResult>,

    results: Vec<PageFromNetwork>,
    servers_contacted: usize,
    servers_responded: usize,
    pages_searched: usize,
}

pub struct ActiveGetEmbedding {
    /** Channel to which we send the results. */
    tx: oneshot::Sender<Vec<f32>>,
}

#[derive(Debug)]
pub struct NetworkSearchResult {
    pub results: Vec<PageFromNetwork>,
    pub servers_contacted: usize,
    pub servers_responded: usize,
    pub pages_searched: usize,
}

pub enum UdpM {
    Search {
        embedding: Vec<f32>,
        distance_limit: Option<f32>,
        tx: oneshot::Sender<NetworkSearchResult>,
    },
    GetEmbedding {
        instance_id: String,
        page_id: usize,
        tx: oneshot::Sender<Vec<f32>>,
    },
    Tick {},
    Announce {},
    Insert {
        page: ExtractedPage,
    },
}

pub struct UdpService {
    pub search_provider_tx: SyncSender<SearchProviderMessage>,
    pub udp_rx: tokio::sync::mpsc::Receiver<UdpM>,
    pub config: Config,
}

impl UdpService {
    pub async fn start(mut self) {
        self.run().await.unwrap();
    }

    async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // let socket = find_port().await?;
        let socket = UdpSocket::bind(&self.config.udp_listen_address).await?; // Random free port.
        let listening_port = socket.local_addr()?.port();
        println!("[UDP] Listening on {}", socket.local_addr()?);

        let mut buf = [0u8; 2000];
        let mut send_buf = Vec::new();

        let mut known_peers: Vec<PeerInfo> = Vec::new();
        let mut active_searches: HashMap<u64, ActiveSearch> = HashMap::new();
        let mut active_get_embeddings: HashMap<u64, ActiveGetEmbedding> = HashMap::new();

        let my_id: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        println!("[UDP] My ID is {}", my_id);

        loop {
            tokio::select! {
                v = socket.recv_from(&mut buf) => {
                    let (len, addr) = v.unwrap();
                    let mut de = Deserializer::new(&buf[..len]);
                    let message: UdpMessage = match Deserialize::deserialize(&mut de) {
                        Ok(m) => m,
                        Err(e) => {
                            println!("Error receiving packet {}", e);
                            continue;
                        }
                    };

                    match message {
                        UdpMessage::Search { search_id, distance_limit, embedding } => {
                            // Slightly hacky way to make sure we don't send searches to ourselves by accident.
                            // TODO: using the ID of a peer for this would be better.
                            if active_searches.contains_key(&search_id) {
                                continue;
                            }

                            let em = Vec::<f32>::from24(&embedding).unwrap();
                            // Send search message to searchprovider.
                            let (otx, orx) = oneshot::channel();
                            self.search_provider_tx
                                .send(SearchProviderMessage::EmbeddingSearch {
                                    otx,
                                    embedding: em.to_vec(),
                                })
                                .unwrap();
                            let result = orx.await.expect("Receiving results");
                            for page in result.pages {
                                if let Some(d) = distance_limit {
                                    if page.distance >= d {
                                        continue; // They are not interested in this one.
                                    }
                                }
                                // Send message back.
                                let m = UdpMessage::Page {
                                    instance_id: my_id.clone(),
                                    page_id: page.page_id,
                                    search_id,
                                    distance: page.distance,
                                    url: page.url,
                                    title: page.title,
                                    text: slice_up_to(&page.text, 500).to_string(),
                                };
                                send_buf.clear();
                                m.serialize(&mut Serializer::new(&mut send_buf)).unwrap();
                                socket.send_to(&send_buf, &addr).await?;
                            }
                        }
                        UdpMessage::Peers { peers } => {
                            known_peers = peers;
                        }
                        UdpMessage::Page { search_id, distance, url, title, text, instance_id, page_id } => {
                            if let Some(q) = active_searches.get_mut(&search_id) {
                                q.results.push(PageFromNetwork {
                                    instance_id,
                                    page_id,
                                    distance,
                                    url,
                                    title,
                                    text });
                            } else {
                                println!("Search result for unknown search {}", search_id);
                            }
                        },
                        UdpMessage::Insert { url_smaz, title_smaz, text_smaz } => {
                            if !self.config.accept_insert {
                                continue;
                            }
                            let url = String::from_utf8_lossy(&smaz::decompress(&url_smaz).unwrap()).to_string();
                            let title = String::from_utf8_lossy(&smaz::decompress(&title_smaz).unwrap()).to_string();
                            let text = String::from_utf8_lossy(&smaz::decompress(&text_smaz).unwrap()).to_string();
                            let mut combined = title.to_string();
                            combined.push(' ');
                            combined.push_str(&text);
                            println!("Received insert for {}", url);
                            self.search_provider_tx.send(SearchProviderMessage::ExtractedPageMessage {
                                page: ExtractedPage {
                                    url,
                                    title,
                                    text,
                                    combined
                                },
                                from_network: true
                            })?;
                        }
                        UdpMessage::Announce {..} => {}
                        UdpMessage::GetEmbedding { search_id, page_id } => {
                            let (otx, orx) = oneshot::channel();
                            self.search_provider_tx.send(SearchProviderMessage::GetEmbedding {
                                page_id,
                                otx,
                            }).unwrap();
                            let em: Vec<f32> = orx.await.unwrap();
                            let m = UdpMessage::Embedding {
                                search_id,
                                embedding: em.to24().as_slice().try_into().unwrap(),
                            };
                            send_buf.clear();
                            m.serialize(&mut Serializer::new(&mut send_buf)).unwrap();
                            socket.send_to(&send_buf, &addr).await?;
                        },
                        UdpMessage::Embedding { search_id, embedding } => {
                            if let Some(x) = active_get_embeddings.remove(&search_id) {
                                x.tx.send(Vec::<f32>::from24(&embedding).unwrap().to_vec()).unwrap();
                            } else {
                                eprintln!("[UDP] Got embedding, but could not find active search {}", search_id);
                            }
                        }
                    }
                }
                v = self.udp_rx.recv() => {
                    // Message to the UDP service
                    let m = v.unwrap();
                    match m {
                        UdpM::Search { embedding, distance_limit, tx } => {
                            let search_id: u64 = rand::thread_rng().gen();
                            println!("[UDP] Search started with id {}", search_id);
                            let mut deadline = Instant::now();
                            if known_peers.len() > 0 {
                                deadline = deadline.checked_add(Duration::from_millis(200)).unwrap();
                            }
                            active_searches.insert(search_id, ActiveSearch {
                                search_id,
                                results: Vec::new(),
                                deadline,
                                tx,
                                servers_contacted: 0,
                                servers_responded: 0,
                                pages_searched: 0,
                            });

                            // Let's fire this one off to our peers.
                            for peer in &known_peers {
                                println!("Sending search to peer {}", peer.instance_id);

                                active_searches.get_mut(&search_id).unwrap().servers_contacted += 1;
                                // TODO: this is a bit optmistic, we should wait until we get a response from the peer.
                                active_searches.get_mut(&search_id).unwrap().pages_searched += peer.pages_indexed;

                                let m = UdpMessage::Search {
                                    search_id,
                                    distance_limit,
                                    embedding: embedding.to24().as_slice().try_into().unwrap(),
                                };
                                send_buf.clear();
                                m.serialize(&mut Serializer::new(&mut send_buf)).unwrap();
                                socket.send_to(&send_buf, &peer.addr).await?;
                            }
                        }
                        UdpM::Tick { } => {
                            let searches_to_remove: Vec<u64> = active_searches.values().filter(|v| Instant::now() > v.deadline).map(|v| v.search_id).collect();
                            for t in searches_to_remove {
                                let removed = active_searches.remove(&t).unwrap();
                                removed.tx.send(NetworkSearchResult {
                                    results: removed.results,
                                    servers_contacted: removed.servers_contacted,
                                    servers_responded: removed.servers_responded,
                                    pages_searched: removed.pages_searched }).unwrap();
                            }
                            // Remove old peers.
                            known_peers.retain(|p| p.last_seen + 300 > now());
                        }
                        UdpM::Announce {} => {
                            #[cfg(feature = "upnp")]
                            if self.config.upnp_enabled {
                                update_upnp(listening_port)?;
                            }

                            // Query the search service for the number of indexed pages.
                            let (otx, orx) = oneshot::channel();
                            self.search_provider_tx.send(SearchProviderMessage::Stats { otx }).unwrap();
                            let stats = orx.await.unwrap();

                            // Announce
                            let announce_message = UdpMessage::Announce {
                                instance_id: my_id.clone(),
                                accept_insert: self.config.accept_insert,
                                pages_indexed: stats.pages_indexed,
                            };
                            send_buf.clear();
                            announce_message
                                .serialize(&mut Serializer::new(&mut send_buf))
                                .unwrap();
                            for tracker in &self.config.trackers {
                                println!("Sending Announce to {}", tracker);
                                if let Err(e) = socket.send_to(&send_buf, tracker).await {
                                    eprintln!("Failed to send announce to {}: {}", tracker, e);
                                }
                            }
                        },
                        UdpM::Insert { page } => {
                            let message = UdpMessage::Insert {
                                url_smaz: smaz::compress(page.url.as_bytes()),
                                title_smaz: smaz::compress(page.title.as_bytes()),
                                text_smaz: smaz::compress(page.text.as_bytes()),
                            };
                            send_buf.clear();
                            message
                                .serialize(&mut Serializer::new(&mut send_buf))
                                .unwrap();
                            println!("Insert message size {}", send_buf.len());

                            // For now, insert with three random peers.
                            let peers_that_accept_insert = known_peers.iter().filter(|p| p.accept_insert).collect::<Vec<&PeerInfo>>();
                            let peers: Vec<&PeerInfo> = { peers_that_accept_insert.choose_multiple(&mut rand::thread_rng(), 3).map(|x| *x).collect() } ;
                            for peer in peers {
                                socket.send_to(&send_buf, &peer.addr).await?;
                            }
                        }
                        UdpM::GetEmbedding { instance_id, page_id, tx } => {
                            if let Some(instance) = known_peers.iter().find(|x| x.instance_id == instance_id) {
                                send_buf.clear();

                                let search_id: u64 = rand::thread_rng().gen();
                                active_get_embeddings.insert(search_id, ActiveGetEmbedding {
                                    tx,
                                });
                                let get_embedding_message = UdpMessage::GetEmbedding {
                                    search_id,
                                    page_id,
                                };
                                get_embedding_message.serialize(&mut Serializer::new(&mut send_buf)).unwrap();
                                socket.send_to(&send_buf, &instance.addr).await?;
                            } else {
                                eprintln!("[UDP] UdpM::GetEmbedding, instance not found {}", instance_id);
                            }
                        }
                    }
                }
            };
        }
    }
}

#[cfg(feature = "upnp")]
fn update_upnp(listening_port: u16) -> anyhow::Result<()> {
    let network_interfaces = NetworkInterface::show().unwrap();
    for itf in network_interfaces.iter() {
        for addr in &itf.addr {
            match addr {
                network_interface::Addr::V4(a) => {
                    let search_options = igd::SearchOptions {
                        bind_addr: SocketAddr::new(IpAddr::V4(a.ip), 0),
                        broadcast_address: SocketAddr::new(
                            IpAddr::V4(Ipv4Addr::new(239, 255, 255, 250)),
                            1900,
                        ),
                        timeout: Some(Duration::from_secs(1)),
                    };
                    let mut local_addr = search_options.bind_addr.clone();
                    local_addr.set_port(listening_port);

                    if let Some(gateway) = igd::search_gateway(search_options).ok() {
                        let ip = gateway.get_external_ip()?;
                        if let Err(e) = gateway.add_port(
                            igd::PortMappingProtocol::UDP,
                            listening_port,
                            SocketAddrV4::new(a.ip, listening_port),
                            600,
                            "DawnSearch",
                        ) {
                            println!(
                                "[UPnP] Could not add mapping from {} to {}: {}",
                                ip, local_addr, e
                            );
                        } else {
                            println!("[UPnP] Mapped {} to {}", ip, local_addr);
                        }
                    }
                }
                network_interface::Addr::V6(_) => {} // ipv6 doesn't support uPNP
            };
        }
    }
    Ok(())
}
