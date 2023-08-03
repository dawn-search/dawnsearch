use anyhow::bail;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::mpsc::SyncSender;
use tokio::net::UdpSocket;
use tokio::sync::oneshot;

use crate::messages::SearchProviderMessage;
use crate::net::udp_messages::UdpMessage;
use crate::vector::{Embedding, ToFrom24};

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

pub async fn udp_server_loop(
    to_search_provider: SyncSender<SearchProviderMessage>,
) -> Result<(), Box<dyn Error>> {
    // let socket = find_port().await?;
    let socket = UdpSocket::bind("0.0.0.0:0").await?; // Random free port.
    println!("Listening on UDP {}", socket.local_addr()?);

    let mut buf = [0u8; 2000];
    let mut send_buf = Vec::new();

    // Announce
    let announce_message = UdpMessage::Announce {
        id: socket.local_addr()?.to_string(), // TEMP
    };
    send_buf.clear();
    announce_message
        .serialize(&mut Serializer::new(&mut send_buf))
        .unwrap();
    socket.send_to(&send_buf, "127.0.0.1:7230").await?;

    while let Ok((len, addr)) = socket.recv_from(&mut buf).await {
        // self.buf contains the data.
        let mut de = Deserializer::new(&buf[..len]);
        let message: UdpMessage = Deserialize::deserialize(&mut de).unwrap();

        match message {
            UdpMessage::Search { embedding } => {
                let em = Embedding::<f32>::from24(embedding.try_into().unwrap()).unwrap();
                println!("Received embedding {:?}", em);
                // Send search message to searchprovider.
                let (otx, orx) = oneshot::channel();
                to_search_provider
                    .send(SearchProviderMessage::EmbeddingSearch {
                        otx,
                        embedding: Box::new(em),
                    })
                    .unwrap();
                let result = orx.await.expect("Receiving results");
                for page in result.pages {
                    // Send message back.
                    let m = UdpMessage::Page {
                        distance: page.distance,
                        url: page.url,
                        title: page.title,
                        text: String::new(), // page.text,
                    };
                    println!("Sending {:?}", m);
                    send_buf.clear();
                    m.serialize(&mut Serializer::new(&mut send_buf)).unwrap();
                    socket.send_to(&send_buf, &addr).await?;
                }
            }
            UdpMessage::Peers { peers } => {
                println!("Received peers {:?}", peers);
            }
            _ => {}
        }
    }

    Ok(())
}
