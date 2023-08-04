use crate::search::messages::SearchProviderMessage;
use crate::search::messages::SearchProviderMessage::*;
use crate::search::search_provider::SearchResult;
use crate::util::slice_up_to;
use std::sync::mpsc::SyncSender;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

fn search_page(results: &str) -> String {
    format!(
        r###"
<html>
<head><title>DawnSearch</title></head>
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

fn format_results(result: &SearchResult) -> String {
    let mut r = String::new();
    r += &format!("<p>Searched {} pages</p>", result.pages_searched);
    for result in &result.pages {
        let url_encoded_u = html_escape::encode_double_quoted_attribute(&result.url);
        let url_encoded = html_escape::encode_text(&result.url);
        let title_encoded = html_escape::encode_text(&result.title);
        let s = slice_up_to(&result.text, 400);
        let text_encoded = html_escape::encode_text(s);
        r += &format!(
            r#"<p><a href="{}">{}</a><br>{:.2} <a href="?s={}">more like this</a> <i>{}</i></p><p>{}...</p>"#,
            url_encoded_u, title_encoded, result.distance, result.id, url_encoded, text_encoded,
        );
    }
    r
}

pub async fn http_server_loop(tx2: SyncSender<SearchProviderMessage>) -> anyhow::Result<()> {
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

        let tx = tx2.clone();

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
            let mut path_query = url.split("?");
            let path = match path_query.next() {
                // ../
                Some(s) => s,
                None => return,
            };
            let kv = if let Some(query) = path_query.next() {
                let mut key_value = query.split("=");
                let key = match key_value.next() {
                    // q
                    Some(s) => s,
                    None => return,
                };
                let value = match key_value.next() {
                    // ...
                    Some(s) => s,
                    None => return,
                };
                Some((key, value))
            } else {
                None
            };

            if path != "/" {
                socket
                    .write_all("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                    .await
                    .unwrap();
                return;
            }

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

            let results = match kv {
                Some((key, value)) => {
                    let (otx, orx) = oneshot::channel();
                    if key == "q" {
                        tx.send(TextSearch {
                            otx,
                            query: urlencoding::decode(value)
                                .expect("Url decode")
                                .to_string()
                                .replace("+", " "),
                        })
                        .unwrap();
                    } else if key == "s" {
                        tx.send(MoreLikeSearch {
                            otx,
                            id: str::parse(value).unwrap(),
                        })
                        .unwrap();
                    }
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
}
