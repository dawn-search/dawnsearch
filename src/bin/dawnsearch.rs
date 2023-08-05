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

use dawnsearch::index::extraction_loop::start_extraction_loop;
use dawnsearch::net::http::http_server_loop;
use dawnsearch::net::udp_service::{UdpM, UdpService};
use dawnsearch::search::messages::SearchProviderMessage;
use dawnsearch::search::messages::SearchProviderMessage::*;
use dawnsearch::search::search_service::SearchService;
use std::env;
use std::time::Duration;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio::{select, signal};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = env::args().collect();

    let should_index = args.iter().any(|x| x == "--index");

    let data_dir = if args.len() > 1 {
        args.iter()
            .last()
            .map(|x| {
                if x.starts_with("--") {
                    None
                } else {
                    Some(x.to_string())
                }
            })
            .flatten()
    } else {
        None
    }
    .unwrap_or(".".to_string());

    let original_shutdown_token = CancellationToken::new();

    let shutdown_token = original_shutdown_token.clone();

    let (search_provider_sender, search_provider_receiver) =
        std::sync::mpsc::sync_channel::<SearchProviderMessage>(2);
    let (udp_tx, udp_rx) = tokio::sync::mpsc::channel::<UdpM>(2);

    let search_provider_tx = search_provider_sender.clone();
    let udp_tx2 = udp_tx.clone();
    tokio::task::spawn_blocking(move || {
        SearchService {
            data_dir,
            shutdown_token,
            search_provider_receiver,
            udp_tx: udp_tx2,
        }
        .start();
    });

    let tx2 = search_provider_sender.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10 * 60)).await;
            if let Err(e) = tx2.send(Save) {
                println!("Saving the index failed: {}", e);
            }
        }
    });

    if should_index {
        let tx2 = search_provider_sender.clone();
        tokio::spawn(async move {
            start_extraction_loop(tx2).await.unwrap();
        });
    }

    let tx2 = search_provider_sender.clone();
    tokio::spawn(async move {
        http_server_loop(tx2).await.unwrap();
    });

    let udp_service = UdpService {
        search_provider_tx: search_provider_sender.clone(),
        udp_rx,
    };
    tokio::spawn(udp_service.start());

    // Timer loop.
    let udp_tx2 = udp_tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(50)).await;
            udp_tx2.send(UdpM::Tick {}).await.unwrap();
        }
    });
    // Announce loop.
    let udp_tx2 = udp_tx.clone();
    tokio::spawn(async move {
        loop {
            udp_tx2.send(UdpM::Announce {}).await.unwrap();
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });

    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    select! {
        _ = sigterm.recv() => println!("Recieved SIGTERM"),
        _ = sigint.recv() => println!("Recieved SIGINT"),
    };

    println!("Shutting down...");

    original_shutdown_token.cancel();
    search_provider_tx.send(Shutdown)?;

    Ok(())
}
