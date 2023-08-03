use arecibo::extraction_loop::start_extraction_loop;
use arecibo::messages::SearchProviderMessage;
use arecibo::messages::SearchProviderMessage::*;
use arecibo::net::http::http_server_loop;
use arecibo::net::udp::udp_server_loop;
use arecibo::search_provider::SearchProvider;
use arecibo::search_provider::SearchResult;
use std::env;
use std::time::Duration;
use tokio::signal;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = env::args().collect();

    let should_index = args.iter().any(|x| x == "--index");

    let data_dir = args
        .iter()
        .last()
        .map(|x| {
            if x.starts_with("--") {
                None
            } else {
                Some(x.to_string())
            }
        })
        .flatten()
        .unwrap_or(".".to_string());

    let original_shutdown_token = CancellationToken::new();

    let shutdown_token = original_shutdown_token.clone();

    let (tx, rx) = std::sync::mpsc::sync_channel::<SearchProviderMessage>(2);
    let search_provider_tx = tx.clone();
    tokio::task::spawn_blocking(move || {
        let mut search_provider = match SearchProvider::new(data_dir, shutdown_token) {
            Err(e) => {
                println!("Failed to load search provider {}", e);
                println!("{}", e.backtrace());
                return;
            }
            Ok(s) => s,
        };
        println!("SearchProvider ready");
        while let Ok(message) = rx.recv() {
            match message {
                TextSearch { otx, query } => {
                    let result = match search_provider.search(&query) {
                        Ok(r) => r,
                        Err(e) => {
                            println!("Failed to perform query: {}", e);
                            SearchResult {
                                pages: Vec::new(),
                                pages_searched: 0,
                            }
                        }
                    };
                    otx.send(result).expect("Send response");
                }
                EmbeddingSearch { otx, embedding } => {
                    let result = match search_provider.search_embedding(&embedding) {
                        Ok(r) => r,
                        Err(e) => {
                            println!("Failed to perform query: {}", e);
                            SearchResult {
                                pages: Vec::new(),
                                pages_searched: 0,
                            }
                        }
                    };
                    otx.send(result).expect("Send response");
                }
                MoreLikeSearch { otx, id } => {
                    let result = match search_provider.search_like(id) {
                        Ok(r) => r,
                        Err(e) => {
                            println!("Failed to perform query: {}", e);
                            SearchResult {
                                pages: Vec::new(),
                                pages_searched: 0,
                            }
                        }
                    };
                    otx.send(result).expect("Send response");
                }
                ExtractedPageMessage { page } => match search_provider.insert(page) {
                    Err(e) => println!("Failed to insert {}", e),
                    _ => {}
                },
                Save => {
                    search_provider.save().unwrap();
                }
                Shutdown => {
                    search_provider.shutdown().unwrap();
                    break;
                }
            }
        }
    });

    let tx2 = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10 * 60)).await;
            if let Err(e) = tx2.send(Save) {
                println!("Saving the index failed: {}", e);
            }
        }
    });

    if should_index {
        let tx2 = tx.clone();
        tokio::spawn(async move {
            start_extraction_loop(tx2).await.unwrap();
        });
    }

    let tx2 = tx.clone();
    tokio::spawn(async move {
        http_server_loop(tx2).await.unwrap();
    });

    let tx2 = tx.clone();
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
