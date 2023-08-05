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

use flate2::read::MultiGzDecoder;
use rand::Rng;
use std::{
    io::{BufRead, BufReader},
    sync::mpsc::SyncSender,
    time::Duration,
};

use crate::search::{messages::SearchProviderMessage, page_source::PageSource};

/** The URL from which we will download the gzipped WARC file list for extracting. */
const WARC_FILE_LIST: &str =
    "https://data.commoncrawl.org/crawl-data/CC-MAIN-2023-23/warc.paths.gz";

/**
 * Download random WARC files from the WARC_FILE_LIST and extract pages from them.
 * The pages will be sent to 'sender' for indexing.
 *
 * This function will run forever.
 */
pub async fn start_extraction_loop(
    sender: SyncSender<SearchProviderMessage>,
) -> anyhow::Result<()> {
    let response = reqwest::blocking::get(WARC_FILE_LIST)?;
    let file_list_reader = BufReader::new(MultiGzDecoder::new(response));
    let files = file_list_reader
        .lines()
        .map(|x| x)
        .collect::<Result<Vec<String>, _>>()?;

    loop {
        let random_file: &str = &files[rand::thread_rng().gen_range(0..files.len())];
        if let Err(e) = extract_file(sender.clone(), random_file).await {
            eprintln!("Error processing {}: {}", random_file, e);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}

/**
 * Extract pages from a single WARC file and send them to 'sender'.
 */
async fn extract_file(
    sender: SyncSender<SearchProviderMessage>,
    random_file: &str,
) -> Result<(), anyhow::Error> {
    println!("Indexing {}", random_file);

    let allow_aws = true;

    if allow_aws && cfg!(s3) {
        #[cfg(feature = "s3")]
        {
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

            tokio::task::spawn_blocking(move || -> Result<(), anyhow::Error> {
                let response = tokio_util::io::SyncIoBridge::new(response_async);

                let mut page_source = PageSource::read_warc_gz(response);

                while let Some(page) = page_source.next()? {
                    sender.send(SearchProviderMessage::ExtractedPageMessage { page })?;
                }
                Ok(())
            })
            .await??;
        }
    } else {
        let mut url_string = "https://data.commoncrawl.org/".to_string();
        url_string.push_str(random_file);

        tokio::task::spawn_blocking(move || -> Result<(), anyhow::Error> {
            let response = reqwest::blocking::get(url_string)?;

            let mut page_source = PageSource::read_warc_gz(response);

            while let Some(page) = page_source.next()? {
                sender.send(SearchProviderMessage::ExtractedPageMessage {
                    page,
                    from_network: false,
                })?;
            }
            Ok(())
        })
        .await??;
    }

    Ok(())
}
