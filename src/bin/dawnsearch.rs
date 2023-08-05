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
use dawnsearch::index::extraction_loop::start_extraction_loop;
use dawnsearch::net::http::http_server_loop;
use dawnsearch::net::udp_service::{UdpM, UdpService};
use dawnsearch::search::messages::SearchProviderMessage;
use dawnsearch::search::messages::SearchProviderMessage::*;
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

    println!("Config file: {}", config_file);
    let settings = Config::builder()
        .add_source(config::File::with_name(&config_file))
        // Add in settings from the environment (with a prefix of DAWNSEARCH)
        // Eg.. `DAWNSEARCH_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(config::Environment::with_prefix("DAWNSEARCH"))
        .build()
        .unwrap();

    let index_cc_enabled = settings.get_bool("index_cc").unwrap_or(false);
    let web_enabled = settings.get_bool("web").unwrap_or(true);
    let web_listen_address = settings
        .get_string("web_listen_address")
        .unwrap_or("0.0.0.0:8080".to_string());
    let udp_enabled = settings.get_bool("udp").unwrap_or(true);
    let udp_listen_address = settings
        .get_string("udp_listen_address")
        .unwrap_or("0.0.0.0:8080".to_string());
    let upnp_enabled = settings.get_bool("upnp").unwrap_or(false);

    let trackers: Vec<String> = settings
        .get_array("trackers")
        .map(|a| a.iter().map(|v| v.clone().into_string().unwrap()).collect())
        .unwrap_or_default();
    let data_dir = settings.get_string("data_dir").unwrap_or(".".to_string());

    println!("Indexing Common Crawl enabled: {}", index_cc_enabled);
    println!("Web enabled: {}", web_enabled);
    println!("Web listen address: {}", web_listen_address);
    println!("UDP enabled: {}", udp_enabled);
    println!("UDP listen address: {}", udp_listen_address);
    println!("UPnP enabled: {}", upnp_enabled);
    println!("Trackers: {:?}", trackers);
    println!("Data directory: {}", data_dir);

    fs::create_dir_all(&data_dir)?;

    let original_shutdown_token = CancellationToken::new();

    let shutdown_token = original_shutdown_token.clone();

    let (search_provider_sender, search_provider_receiver) =
        std::sync::mpsc::sync_channel::<SearchProviderMessage>(2);
    let (udp_tx, mut udp_rx) = tokio::sync::mpsc::channel::<UdpM>(2);

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

    if index_cc_enabled {
        let tx2 = search_provider_sender.clone();
        tokio::spawn(async move {
            start_extraction_loop(tx2).await.unwrap();
        });
    }

    if web_enabled {
        let tx2 = search_provider_sender.clone();
        tokio::spawn(async move {
            http_server_loop(tx2, web_listen_address).await.unwrap();
        });
    }

    if udp_enabled {
        let udp_service = UdpService {
            search_provider_tx: search_provider_sender.clone(),
            udp_rx,
            upnp_enabled,
            trackers,
            udp_listen_address,
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
    } else {
        // Discard messages.
        tokio::spawn(async move {
            loop {
                udp_rx.recv().await.unwrap();
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
    search_provider_tx.send(Shutdown)?;

    Ok(())
}
