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

use dawnsearch::net::udp_messages::{PeerInfo, UdpMessage};
use dawnsearch::util::now;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:7230").await?;

    let mut buf = [0u8; 2000];
    let mut send_buf: Vec<u8> = Vec::new();

    // TODO: probably replace this by postgres.
    let mut peers: HashMap<String, PeerInfo> = HashMap::new();

    while let Ok((len, addr)) = socket.recv_from(&mut buf).await {
        let mut de = Deserializer::new(&buf[..len]);
        let message: UdpMessage = Deserialize::deserialize(&mut de).unwrap();
        match message {
            UdpMessage::Announce { id } => {
                println!("Announce ID {} addr {}", id, addr);
                peers.insert(
                    id.clone(),
                    PeerInfo {
                        id: id.clone(),
                        addr: addr.to_string(),
                        last_seen: now(),
                    },
                );
                let all: Vec<PeerInfo> = peers
                    .values()
                    .filter(|p| p.id != id && now() - p.last_seen < 10 * 60)
                    .map(|x| x.clone())
                    .collect();
                let response = UdpMessage::Peers { peers: all };
                send_buf.clear();
                response
                    .serialize(&mut Serializer::new(&mut send_buf))
                    .unwrap();
                println!("");
                println!("Data: {} {:?}", send_buf.len(), send_buf);
                // TODO: possible write amplification.
                // TODO: split into multiple packets.
                socket.send_to(&send_buf, addr).await?;
            }
            _ => {}
        }
    }

    Ok(())
}
