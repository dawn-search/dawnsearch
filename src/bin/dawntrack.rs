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

use anyhow::bail;
use config::Config;
use dawnsearch::net::udp_messages::{PeerInfo, UdpMessage};
use dawnsearch::util::now;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 2 {
        bail!("Usage: dawntrack [config file]");
    }

    let config_file = if args.len() == 2 {
        args[1].clone()
    } else {
        "DawnTrack.toml".to_string()
    };

    println!("Config file: {}", config_file);
    let settings = Config::builder()
        .add_source(config::File::with_name(&config_file))
        // Add in settings from the environment (with a prefix of DAWNSEARCH)
        // Eg.. `DAWNSEARCH_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(config::Environment::with_prefix("DAWNTRACK"))
        .build()
        .unwrap();

    let udp_listen_address = settings
        .get_string("udp_listen_address")
        .unwrap_or("0.0.0.0:7230".to_string());
    let external_address: Option<String> = settings.get_string("external_address").ok();

    let socket = UdpSocket::bind(udp_listen_address).await?;

    let mut buf = [0u8; 2000];
    let mut send_buf: Vec<u8> = Vec::new();

    // TODO: probably replace this by postgres.
    let mut peers: HashMap<String, PeerInfo> = HashMap::new();

    while let Ok((len, mut addr)) = socket.recv_from(&mut buf).await {
        let mut de = Deserializer::new(&buf[..len]);
        let message: UdpMessage = Deserialize::deserialize(&mut de).unwrap();
        match message {
            UdpMessage::Announce { id, accept_insert } => {
                println!("Announce ID {} addr {}", id, addr);
                if let Some(x) = &external_address {
                    if addr.ip().is_loopback() {
                        addr.set_ip(x.parse().unwrap());
                        println!("Address replaced by {}", addr);
                    }
                }
                peers.insert(
                    id.clone(),
                    PeerInfo {
                        id: id.clone(),
                        addr: addr.to_string(),
                        last_seen: now(),
                        accept_insert,
                    },
                );
                let all: Vec<PeerInfo> = peers
                    .values()
                    .filter(|p| p.id != id && now() - p.last_seen < 10 * 60)
                    .map(|x| x.clone())
                    .collect();
                for chunk in all.chunks(25) {
                    // We can probably fit 40 but let's be careful.
                    let response = UdpMessage::Peers {
                        peers: chunk.to_vec(),
                    };
                    send_buf.clear();
                    response
                        .serialize(&mut Serializer::new(&mut send_buf))
                        .unwrap();
                    // TODO: possible write amplification.
                    socket.send_to(&send_buf, addr).await?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}
