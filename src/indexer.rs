use std::{
    io::{BufRead, BufReader},
    sync::mpsc::SyncSender,
};

use flate2::read::MultiGzDecoder;
use rand::Rng;

use crate::page_source::PageSource;

use crate::messages::SearchProviderMessage;
use crate::messages::SearchProviderMessage::*;

const WARC_FILE_LIST: &str =
    "https://data.commoncrawl.org/crawl-data/CC-MAIN-2023-23/warc.paths.gz";

pub async fn start_index_loop(sender: SyncSender<SearchProviderMessage>) -> anyhow::Result<()> {
    let response = reqwest::blocking::get(WARC_FILE_LIST)?;
    let file_list_reader = BufReader::new(MultiGzDecoder::new(response));
    let files: Vec<String> = file_list_reader.lines().map(|x| x.unwrap()).collect();

    loop {
        let random_file = &files[rand::thread_rng().gen_range(0..files.len())];
        // let random_file = "crawl-data/CC-MAIN-2023-23/segments/1685224645089.3/warc/CC-MAIN-20230530032334-20230530062334-00157.warc.gz";

        println!("Indexing {}", random_file);

        let config = ::aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&config);

        let response_async = client
            .get_object()
            .bucket("commoncrawl")
            .key(random_file)
            .send()
            .await?
            .body
            .into_async_read();

        let sender = sender.clone();

        let mut url_string = "https://data.commoncrawl.org/".to_string();
        url_string.push_str(random_file);

        tokio::task::spawn_blocking(move || {
            let response = tokio_util::io::SyncIoBridge::new(response_async);

            // let response = reqwest::blocking::get(url_string).unwrap();

            let mut page_source = PageSource::read_warc_gz(response);

            while let Some(page) = page_source.next().unwrap() {
                sender.send(ExtractedPageMessage { page }).unwrap();
            }
        })
        .await
        .unwrap();
    }
}
