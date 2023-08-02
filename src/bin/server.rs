use std::str;
use std::{self};

use arecibo::extraction_loop::start_extraction_loop;
use arecibo::messages::SearchProviderMessage;
use arecibo::messages::SearchProviderMessage::*;
use arecibo::search_provider::SearchProvider;
use arecibo::search_provider::SearchResult;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

fn search_page(results: &str) -> String {
    format!(
        r###"
<html>
<body style="margin: 2em">
<form method="get">
<input name="q" id="searchbox">
<input type="submit" value="Search">
</form>
{results:}
<script>
document.getElementById("searchbox").focus();
</script>
</body>
</html>
"###
    )
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let original_shutdown_token = CancellationToken::new();

    let shutdown_token = original_shutdown_token.clone();

    let (tx, rx) = std::sync::mpsc::sync_channel::<SearchProviderMessage>(2);
    let search_provider_tx = tx.clone();
    tokio::task::spawn_blocking(move || {
        let mut search_provider = SearchProvider::new(shutdown_token).unwrap();
        println!("SearchProvider ready");
        while let Ok(message) = rx.recv() {
            match message {
                SearchRequestMessage { otx, query } => {
                    let result = search_provider.search(&query).unwrap();
                    otx.send(result).expect("Send response");
                }
                ExtractedPageMessage { page } => {
                    search_provider.insert(page).unwrap();
                }
                Shutdown {} => {
                    search_provider.shutdown();
                    break;
                }
            }
        }
    });

    let tx2 = tx.clone();
    tokio::spawn(async move {
        start_extraction_loop(tx2).await.unwrap();
    });

    tokio::spawn(async move {
        let addr = "127.0.0.1:8080";
        // Next up we create a TCP listener which will listen for incoming
        // connections. This TCP listener is bound to the address we determined
        // above and must be associated with an event loop.
        let listener = TcpListener::bind(&addr).await.unwrap();
        println!("Listening on: {}", addr);

        loop {
            // Asynchronously wait for an inbound socket.
            let (socket, _) = match listener.accept().await {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("Error on accept {:?}", e);
                    continue;
                }
            };

            // And this is where much of the magic of this server happens. We
            // crucially want all clients to make progress concurrently, rather than
            // blocking one on completion of another. To achieve this we use the
            // `tokio::spawn` function to execute the work in the background.
            //
            // Essentially here we're executing a new task to run concurrently,
            // which will allow all of our clients to be processed concurrently.

            let tx = tx.clone();

            tokio::spawn(async move {
                let mut socket = BufReader::new(socket);

                let mut request = String::new();
                socket.read_line(&mut request).await.unwrap();

                let mut parts = request.split(" ");
                let method = match parts.next() {
                    Some(s) => s,
                    None => return,
                };
                if method != "GET" {
                    return;
                }
                let url = match parts.next() {
                    Some(s) => s,
                    None => return,
                };
                let mut url_parts = url.split("?q=");
                let path = match url_parts.next() {
                    Some(s) => s,
                    None => return,
                };

                if path != "/" {
                    socket
                        .write_all("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                        .await
                        .unwrap();
                    return;
                }

                let q = url_parts.next();

                let mut line = String::new();
                while socket.read_line(&mut line).await.is_ok() {
                    if line == "\r\n" {
                        break; // Found the empty line signaling the end of the headers.
                    }
                    line.clear();
                }

                socket
                    .write_all(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n"
                            .as_bytes(),
                    )
                    .await
                    .unwrap();

                let results = match q {
                    Some(query) => {
                        let (otx, orx) = oneshot::channel();
                        tx.send(SearchRequestMessage {
                            otx,
                            query: urlencoding::decode(query)
                                .expect("Url decode")
                                .to_string()
                                .replace("+", " "),
                        })
                        .unwrap();
                        let result = orx.await.expect("Receiving results");
                        format_results(&result)
                    }
                    None => String::new(),
                };
                socket
                    .write_all(search_page(&results).as_bytes())
                    .await
                    .unwrap();
            });
        }
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

fn format_results(result: &SearchResult) -> String {
    let mut r = String::new();
    r += &format!("<p>Searched {} pages</p>", result.pages_searched);
    for result in &result.pages {
        let url_encoded_u = html_escape::encode_double_quoted_attribute(&result.url);
        let url_encoded = html_escape::encode_text(&result.url);
        let title_encoded = html_escape::encode_text(&result.title);
        r += &format!(
            r#"<p><a href="{}">{}</a><br>{:.2} <i>{}</i></p>"#,
            url_encoded_u, title_encoded, result.distance, url_encoded
        );
    }
    r
}
