use std::env;
use std::iter::zip;
use std::time::Instant;
use std::{self, fs};
use std::{str, usize};

use cxx::UniquePtr;
use indicatif::{ProgressBar, ProgressStyle};
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use usearch::ffi::{new_index, Index, IndexOptions, MetricKind, ScalarKind};

use arecibo::document_embeddings::DocumentEmbeddings;
use arecibo::vector::{Embedding, EM_LEN};

fn search_page(results: &str) -> String {
    format!(
        r###"
<html>
<body>
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

struct SearchRequestMessage {
    otx: tokio::sync::oneshot::Sender<SearchRequestResponse>,
    query: String,
}

#[derive(Debug)]
struct SearchRequestResponse {
    results: Vec<SearchResult>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = env::args().collect();
    let warc_dir = args[1].clone();

    let addr = "127.0.0.1:8080";

    // Next up we create a TCP listener which will listen for incoming
    // connections. This TCP listener is bound to the address we determined
    // above and must be associated with an event loop.
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on: {}", addr);

    let (tx, rx) = std::sync::mpsc::channel::<SearchRequestMessage>();
    tokio::task::spawn_blocking(move || {
        let search_provider = SearchProvider::load(&warc_dir).unwrap();
        println!("SearchProvider ready");
        while let Ok(x) = rx.recv() {
            let results = search_provider.search(&x.query).unwrap();
            x.otx
                .send(SearchRequestResponse { results })
                .expect("Send response");
            println!("Results sent");
        }
        println!("Search thread finished");
    });

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
                println!("Request: {:?}", line);
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
                        query: query.to_string(),
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
            r#"<p><a href="{}">{}</a><br>{:.2} {}</p>"#,
            url_encoded_u, title_encoded, result.distance, url_encoded
        );
    }
    r
}

#[derive(Debug)]
struct SearchResult {
    distance: f32,
    url: String,
    title: String,
}

struct SearchProvider {
    model: SentenceEmbeddingsModel,
    document_embeddings: DocumentEmbeddings,
    index: UniquePtr<Index>,
}

impl SearchProvider {
    fn load(warc_dir: &str) -> Result<SearchProvider, anyhow::Error> {
        let start = Instant::now();

        print!("Loading model...");

        let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
            .with_device(tch::Device::Cpu)
            .create_model()?;

        let duration = start.elapsed();
        println!(" {} ms", duration.as_millis());

        let document_embeddings = DocumentEmbeddings::load(&warc_dir)?;
        let total_documents = document_embeddings.len();

        let options = IndexOptions {
            dimensions: EM_LEN,
            metric: MetricKind::IP,
            quantization: ScalarKind::F8,
            connectivity: 0,
            expansion_add: 0,
            expansion_search: 0,
        };

        let index = new_index(&options).unwrap();

        let index_path = "index.usearch";
        if !fs::metadata(index_path).is_ok() || !index.view(index_path).is_ok() {
            println!("Recalculating index...");
            index.reserve(document_embeddings.len())?;

            let progress = ProgressBar::new(total_documents as u64);
            progress.set_style(
                ProgressStyle::with_template("[{elapsed_precise}] {bar}{pos:>7}/{len:7} {msg}")
                    .unwrap(),
            );
            let mut searched_pages_count = 0;
            for page in 0..document_embeddings.files() {
                for entry in 0..document_embeddings.entries(page) {
                    progress.set_position(searched_pages_count);
                    let p = document_embeddings.entry(page, entry);
                    index.add(searched_pages_count, &p.vector)?;
                    searched_pages_count += 1;
                }
            }
            progress.finish();
            index.save(index_path)?;
        } else {
            println!("Loaded index {}", index_path);
        }
        Ok(SearchProvider {
            model,
            document_embeddings,
            index,
        })
    }

    fn search(&self, query: &str) -> Result<Vec<SearchResult>, anyhow::Error> {
        let mut result = Vec::new();

        let q = &self.model.encode(&[query]).unwrap()[0];
        let query_embedding: &Embedding<f32> = q.as_slice().try_into().unwrap();

        let start = Instant::now();

        // Read back the tags
        let results = self.index.search(query_embedding, 20).unwrap();

        let duration = start.elapsed();

        for (distance, id) in zip(results.distances, results.labels) {
            let (file, entry) = self.document_embeddings.linear_to_segmented(id as usize);

            // let e = document_embeddings.entry(file, entry);
            let url: &[u8] = self.document_embeddings.url(file, entry);
            let title: &[u8] = self.document_embeddings.title(file, entry);
            println!(
                "{:.2}: {} - {}",
                distance,
                unsafe { str::from_utf8_unchecked(title) },
                unsafe { str::from_utf8_unchecked(url) }
            );
            result.push(SearchResult {
                distance,
                url: str::from_utf8(url)?.to_owned(),
                title: str::from_utf8(title)?.to_owned(),
            });
        }
        println!("Search completed in {} us", duration.as_micros(),);

        Ok(result)
    }
}