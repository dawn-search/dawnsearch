use arecibo::extraction_loop::start_extraction_loop;
use arecibo::messages::SearchProviderMessage;
use arecibo::messages::SearchProviderMessage::*;
use arecibo::net::http::http_server_loop;
use arecibo::net::udp::udp_server_loop;
use arecibo::search::search_service::SearchService;
use std::env;
use std::time::Duration;
use tokio::signal;
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
    let search_provider_tx = search_provider_sender.clone();
    tokio::task::spawn_blocking(move || {
        SearchService {
            data_dir,
            shutdown_token,
            search_provider_receiver,
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

    let tx2 = search_provider_sender.clone();
    tokio::spawn(async move {
        udp_server_loop(tx2).await.unwrap();
    });

    match signal::ctrl_c().await {
        Ok(()) => {}
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
        }
    }

    println!("");
    println!("Ctrl-C received, shutting down...");

    original_shutdown_token.cancel();
    search_provider_tx.send(Shutdown)?;

    Ok(())
}
