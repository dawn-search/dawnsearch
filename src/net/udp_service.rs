use crate::net::udp_messages::{PeerInfo, UdpMessage};
use crate::search::messages::SearchProviderMessage;
use crate::search::vector::{Embedding, ToFrom24};
use crate::util::slice_up_to;
use anyhow::bail;
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
    pub distance: f32,
    pub url: String,
    pub title: String,
    pub text: String,
}

pub struct ActiveSearch {
    search_id: u64,
    results: Vec<PageFromNetwork>,
    start: Instant,
    /** Channel to which we send the results. */
    tx: oneshot::Sender<Vec<PageFromNetwork>>,
}

pub enum UdpM {
    Search {
        embedding: Vec<f32>,
        tx: oneshot::Sender<Vec<PageFromNetwork>>,
    },
    Tick {},
    Announce {},
}

pub struct UdpService {
    pub search_provider_tx: SyncSender<SearchProviderMessage>,
    pub udp_rx: tokio::sync::mpsc::Receiver<UdpM>,
}

impl UdpService {
    pub async fn start(mut self) {
        self.run().await.unwrap();
    }

    async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // let socket = find_port().await?;
        let socket = UdpSocket::bind("0.0.0.0:0").await?; // Random free port.
        let listening_port = socket.local_addr()?.port();
        println!("Listening on UDP {}", socket.local_addr()?);

        let mut buf = [0u8; 2000];
        let mut send_buf = Vec::new();

        let mut known_peers: Vec<PeerInfo> = Vec::new();
        let mut active_searches: HashMap<u64, ActiveSearch> = HashMap::new();

        loop {
            tokio::select! {
                v = socket.recv_from(&mut buf) => {
                    let (len, addr) = v.unwrap();
                    let mut de = Deserializer::new(&buf[..len]);
                    let message: UdpMessage = Deserialize::deserialize(&mut de).unwrap();

                    match message {
                        UdpMessage::Search { search_id, embedding } => {
                            let em = Embedding::<f32>::from24(embedding.try_into().unwrap()).unwrap();
                            println!("Received embedding {:?}", em);
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
                                // Send message back.
                                let m = UdpMessage::Page {
                                    search_id,
                                    distance: page.distance,
                                    url: page.url,
                                    title: page.title,
                                    text: slice_up_to(&page.text, 500).to_string(),
                                };
                                println!("Sending {:?}", m);
                                send_buf.clear();
                                m.serialize(&mut Serializer::new(&mut send_buf)).unwrap();
                                socket.send_to(&send_buf, &addr).await?;
                            }
                        }
                        UdpMessage::Peers { peers } => {
                            known_peers = peers;
                        }
                        UdpMessage::Page { search_id, distance, url, title, text } => {
                            if let Some(q) = active_searches.get_mut(&search_id) {
                                q.results.push(PageFromNetwork { distance, url, title, text });
                            } else {
                                println!("Search result for unknown search {}", search_id);
                            }
                        }
                        x => {
                            println!("Unhandled UDP message: {:?}", x);
                        }
                    }
                }
                v = self.udp_rx.recv() => {
                    // Message to the UDP service
                    let m = v.unwrap();
                    match m {
                        UdpM::Search { embedding, tx } => {
                            let search_id: u64 = rand::thread_rng().gen();
                            println!("[UDP] Search started with id {}", search_id);
                            active_searches.insert(search_id, ActiveSearch {
                                search_id,
                                results: Vec::new(),
                                start: Instant::now(),
                                tx,
                            });

                            let em: Embedding<f32> = embedding.as_slice().try_into().unwrap();

                            // Let's fire this one off to our peers.
                            for peer in &known_peers {
                                println!("Sending search to peer {}", peer.id);

                                let m = UdpMessage::Search {
                                    search_id,
                                    embedding: em.to24().as_slice().try_into().unwrap(),
                                };
                                send_buf.clear();
                                m.serialize(&mut Serializer::new(&mut send_buf)).unwrap();
                                socket.send_to(&send_buf, &peer.addr).await?;
                            }
                        }
                        UdpM::Tick { } => {
                            let to_remove: Vec<u64> = active_searches.values().filter(|v| v.start.elapsed() > Duration::from_millis(200)).map(|v| v.search_id).collect();
                            for t in to_remove {
                                let removed = active_searches.remove(&t).unwrap();
                                println!("Search time expired, sending {:?}", removed.results);
                                removed.tx.send(removed.results).unwrap();
                            }
                        }
                        UdpM::Announce {} => {
                            #[cfg(feature = "upnp")]
                            update_upnp(listening_port)?;

                            // Announce
                            let announce_message = UdpMessage::Announce {
                                id: socket.local_addr()?.to_string(), // TEMP
                            };
                            send_buf.clear();
                            announce_message
                                .serialize(&mut Serializer::new(&mut send_buf))
                                .unwrap();
                            socket.send_to(&send_buf, "127.0.0.1:7230").await?;
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
                            "Arecibo",
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
