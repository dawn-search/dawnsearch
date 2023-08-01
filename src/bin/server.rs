use std::env;
use std::str;
use std::{self};

use arecibo::indexer::start_index_loop;
use arecibo::messages::SearchProviderMessage;
use arecibo::messages::SearchProviderMessage::*;
use arecibo::messages::SearchRequestResponse;
use arecibo::search_provider::SearchProvider;
use arecibo::search_provider::SearchResult;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

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
    let args: Vec<String> = env::args().collect();
    let warc_dir = args[1].clone();

    let (tx, rx) = std::sync::mpsc::sync_channel::<SearchProviderMessage>(2);
    tokio::task::spawn_blocking(move || {
        // let mut search_provider = SearchProvider::load(&warc_dir).unwrap();
        let mut search_provider = SearchProvider::new().unwrap();
        println!("SearchProvider ready");
        while let Ok(x) = rx.recv() {
            match x {
                SearchRequestMessage { otx, query } => {
                    let results = search_provider.search(&query).unwrap();
                    otx.send(SearchRequestResponse { results })
                        .expect("Send response");
                }
                ExtractedPageMessage { page } => {
                    search_provider.insert(page).unwrap();
                }
            }
        }
    });

    let tx2 = tx.clone();
    tokio::spawn(async move {
        start_index_loop(tx2).await.unwrap();
    });

    let addr = "127.0.0.1:8080";
    // Next up we create a TCP listener which will listen for incoming
    // connections. This TCP listener is bound to the address we determined
    // above and must be associated with an event loop.
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on: {}", addr);

    loop {
        // Asynchronously wait for an inbound socket.
        let (socket, _) = listener.accept().await?;

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
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n".as_bytes(),
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
                    let r = orx.await.expect("Receiving results");
                    format_results(&r.results)
                }
                None => String::new(),
            };
            socket
                .write_all(search_page(&results).as_bytes())
                .await
                .unwrap();
        });
    }
}

fn format_results(results: &[SearchResult]) -> String {
    let mut r = String::new();
    for result in results {
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
