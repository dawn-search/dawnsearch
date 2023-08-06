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

use crate::config::Config;
use crate::net::web_content::{format_results, main_page, results_page};
use crate::search::messages::SearchProviderMessage;
use crate::search::messages::SearchProviderMessage::*;
use std::sync::mpsc::SyncSender;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

pub async fn http_server_loop(
    tx2: SyncSender<SearchProviderMessage>,
    config: Config,
) -> anyhow::Result<()> {
    // Next up we create a TCP listener which will listen for incoming
    // connections. This TCP listener is bound to the address we determined
    // above and must be associated with an event loop.
    let listener = TcpListener::bind(&config.web_listen_address).await.unwrap();
    println!("[Web] Listening on: {}", &config.web_listen_address);

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

            if path == "/robots.txt" {
                socket
                    .write_all(
                        "HTTP/1.1 200 OK\r\n\r\nUser-agent: *\r\nDisallow: /?\r\n".as_bytes(),
                    )
                    .await
                    .unwrap();
                return;
            }

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

            let mut query = String::new();
            let results = match kv {
                Some((key, value)) => {
                    let (otx, orx) = oneshot::channel();
                    if key == "q" {
                        query = urlencoding::decode(value)
                            .expect("Url decode")
                            .to_string()
                            .replace("+", " ");
                        tx.send(TextSearch {
                            otx,
                            query: query.clone(),
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
                    Some(format_results(&result))
                }
                None => None,
            };

            socket
                .write_all(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n".as_bytes(),
                )
                .await
                .unwrap();
            if let Some(r) = results {
                socket
                    .write_all(results_page(&query, &r).as_bytes())
                    .await
                    .unwrap();
            } else {
                socket.write_all(main_page().as_bytes()).await.unwrap();
            }
        });
    }
}
