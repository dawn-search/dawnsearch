use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::mpsc::SyncSender;
use tokio::net::UdpSocket;
use tokio::sync::oneshot;

use crate::messages::SearchProviderMessage;
use crate::net::udp_messages::UdpMessage;
use crate::vector::{Embedding, ToFrom24};

const UDP_PORT: u32 = 7231; // Looks like nobody is using this one yet.

pub async fn run_udp_server(tx: SyncSender<SearchProviderMessage>) -> Result<(), Box<dyn Error>> {
    let addr = format!("0.0.0.0:{}", UDP_PORT);

    let socket = UdpSocket::bind(&addr).await?;
    println!("Listening on UDP {}", socket.local_addr()?);

    let mut buf = [0u8; 2000];
    let mut send_buf = Vec::new();
    while let Ok((len, addr)) = socket.recv_from(&mut buf).await {
        // self.buf contains the data.
        let mut de = Deserializer::new(&buf[..len]);
        let actual: UdpMessage = Deserialize::deserialize(&mut de).unwrap();

        match actual {
            UdpMessage::Search { embedding } => {
                let em = Embedding::<f32>::from24(embedding.try_into().unwrap()).unwrap();
                println!("Received embedding {:?}", em);
                // Send search message to searchprovider.
                let (otx, orx) = oneshot::channel();
                tx.send(SearchProviderMessage::EmbeddingSearch {
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
            _ => {}
        }
    }

    Ok(())
}
