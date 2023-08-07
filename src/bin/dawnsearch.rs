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
use dawnsearch::config::Config;
use dawnsearch::index::extraction_service::start_extraction_service;
use dawnsearch::net::http_service::start_http_service;
use dawnsearch::net::udp_service::{UdpMsg, UdpService};
use dawnsearch::search::search_msg::SearchMsg;
use dawnsearch::search::search_msg::SearchMsg::*;
use dawnsearch::search::search_service::SearchService;
use std::time::Duration;
use std::{env, fs};
use tokio::select;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 2 {
        bail!("Usage: dawnsearch [config file]");
    }

    let config_file = if args.len() == 2 {
        args[1].clone()
    } else {
        "DawnSearch.toml".to_string()
    };

    let config = Config::load(&config_file);
    config.print();

    fs::create_dir_all(&config.data_dir)?;

    let original_shutdown_token = CancellationToken::new();

    let shutdown_token = original_shutdown_token.clone();

    let (search_tx, search_rx) = std::sync::mpsc::sync_channel::<SearchMsg>(2);
    let (udp_tx, mut udp_rx) = tokio::sync::mpsc::channel::<UdpMsg>(2);

    let udp_tx2 = udp_tx.clone();
    let config2 = config.clone();
    let search_tx2 = search_tx.clone();
    tokio::task::spawn_blocking(move || {
        SearchService {
            config: config2,
            shutdown_token,
            search_rx,
            search_tx: search_tx2,
            udp_tx: udp_tx2,
        }
        .start();
    });

    let tx2 = search_tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10 * 60)).await;
            if let Err(e) = tx2.send(Save) {
                println!("Saving the index failed: {}", e);
            }
        }
    });

    if config.index_cc_enabled {
        let tx2 = search_tx.clone();
        tokio::spawn(async move {
            start_extraction_service(tx2).await.unwrap();
        });
    }

    let config2 = config.clone();
    if config.web_enabled {
        let tx2 = search_tx.clone();
        tokio::spawn(async move {
            start_http_service(tx2, config2).await.unwrap();
        });
    }

    if config.udp_enabled {
        let udp_service = UdpService {
            search_tx: search_tx.clone(),
            udp_rx,
            config,
        };
        tokio::spawn(udp_service.start());

        // Timer loop.
        let udp_tx2 = udp_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(50)).await;
                udp_tx2.send(UdpMsg::Tick {}).await.unwrap();
            }
        });
        // Announce loop.
        let udp_tx2 = udp_tx.clone();
        tokio::spawn(async move {
            loop {
                udp_tx2.send(UdpMsg::Announce {}).await.unwrap();
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
    } else {
        // Discard messages.
        tokio::spawn(async move {
            loop {
                let discarded = udp_rx.recv().await.unwrap();
                if config.debug > 0 {
                    println!("[Main] Discarding packet for UDP system {:?}", discarded);
                }
            }
        });
    }

    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    select! {
        _ = sigterm.recv() => println!("Recieved SIGTERM"),
        _ = sigint.recv() => println!("Recieved SIGINT"),
    };

    println!("Shutting down...");

    original_shutdown_token.cancel();
    search_tx.send(Shutdown)?;

    Ok(())
}
