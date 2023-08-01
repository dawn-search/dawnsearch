use std;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use flate2::read::MultiGzDecoder;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use markup5ever_rcdom::RcDom;
use rust_bert::pipelines::sentence_embeddings::SentenceEmbeddingsModel;
use std::io::Read;
use url::Url;
use whichlang::detect_language;
use whichlang::Lang;

use crate::extract::extract;
use crate::extract::extract_text;
use crate::util::slice_up_to;
use crate::vector::Embedding;

#[derive(Debug)]
#[repr(C)]
pub struct PageEntry {
    pub url_pos: u64,
    pub title_pos: u64,
    pub vector: Embedding<f32>,
    pub url_len: u64,
    pub title_len: u64,
}

struct RecordOwned {
    uri: String,
    warc_type: String,
    payload_type: String,
    body: String,
}

fn read_record(reader: &mut dyn io::BufRead) -> Result<Option<RecordOwned>, std::io::Error> {
    let mut content_length = 0;
    let mut uri = String::new();
    let mut warc_type = String::new();
    let mut payload_type = String::new();

    // Read headers
    for line in reader.lines() {
        let line = line?;
        if line == "" {
            if payload_type != "text/html" {
                // Discard content.
                io::copy(&mut reader.take(content_length), &mut io::sink())?;
                return Ok(Some(RecordOwned {
                    uri: uri,
                    warc_type: warc_type,
                    payload_type,
                    body: String::new(),
                }));
            }
            let mut request_response = Vec::new();
            let mut payload_reader = reader.take(content_length as u64);
            payload_reader.read_to_end(&mut request_response)?;

            let rr_str = String::from_utf8_lossy(&request_response);

            // Now, we need to split the headers and the response
            let mut s = rr_str.splitn(2, "\r\n\r\n");
            s.next().unwrap();
            if let Some(body) = s.next() {
                return Ok(Some(RecordOwned {
                    uri: uri,
                    warc_type: warc_type,
                    payload_type,
                    body: body.to_owned(),
                }));
            }
            return Ok(Some(RecordOwned {
                uri: uri,
                warc_type: warc_type,
                payload_type,
                body: String::new(),
            }));
        }
        let mut kv = line.splitn(2, ':').map(|s| s.trim());
        let key = kv.next().unwrap();
        if let Some(value) = kv.next() {
            if key == "Content-Length" {
                content_length = value.parse().unwrap();
            }
            if key == "WARC-Target-URI" {
                uri = value.to_owned();
            }
            if key == "WARC-Type" {
                warc_type = value.to_owned();
            }
            if key == "WARC-Identified-Payload-Type" {
                payload_type = value.to_owned();
            }
        }
    }
    Ok(None)
}

// From https://stackoverflow.com/questions/28127165/how-to-convert-struct-to-u8
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
}

pub fn extract_records_and_add_to_index(
    input: &mut dyn Read,
    filename: &PathBuf,
    model: &SentenceEmbeddingsModel,
) -> io::Result<()> {
    const PER_THREAD_BUF_SIZE: usize = 16 * 1024 * 1024;
    let mut reader = io::BufReader::with_capacity(PER_THREAD_BUF_SIZE, MultiGzDecoder::new(input));

    let mut output_file_name = filename.clone();
    output_file_name.set_extension("emb");
    let mut output_writer = File::create(output_file_name)?;

    let mut url_name = filename.clone();
    url_name.set_extension("url");
    let mut url_writer = File::create(url_name)?;

    let mut title_name = filename.clone();
    title_name.set_extension("title");
    let mut title_writer = File::create(title_name)?;

    let mut count = 0;
    let mut added = 0;

    let mut url_pos: u64 = 0;
    let mut title_pos: u64 = 0;

    let progress = ProgressBar::new(36000);
    progress.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] {bar}{pos:>7}/{len:7} {msg}").unwrap(),
    );
    let mut start = Instant::now();
    let mut speed = 0.0f32;
    let mut per_embedding = 0.0f32;

    while let Some(record) = read_record(&mut reader)? {
        progress.set_position(count);
        if record.warc_type != "conversion" && record.warc_type != "response" {
            continue;
        }
        if record.payload_type != "text/html" {
            continue;
        }
        // TODO: for requests there are a bunch of headers... not sure our extractor will like that.
        count += 1;

        let uri = record.uri;
        if uri.contains("?") || uri.contains("%") {
            continue;
        }

        let body = record.body;

        if body.len() < 500 {
            continue;
        }

        // Doing early language detection does not work, too much HTML noise.

        // 16 sec

        let mut body_slice = slice_up_to(&body, 1024 * 250).as_bytes();

        let url = Url::parse(&uri).unwrap();

        let mut dom = match parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut body_slice)
        {
            Ok(dom) => dom,
            Err(e) => {
                println!("Failed to read {}: {}", e, url);
                continue;
            }
        };

        let (cleaned_document, title) = extract(&mut dom, &url);
        let mut clean: String = String::new();
        extract_text(&cleaned_document, &mut clean, true);

        // 25 seconds (with 10kb payload)

        if clean.len() < 200 {
            continue;
        }

        let title = slice_up_to(&title, 200);

        let lang = detect_language(slice_up_to(&clean, 2048));
        if lang != Lang::Eng {
            continue;
        }

        // 25 seconds

        let embedding = &model.encode(&[slice_up_to(&clean, 2048)]).unwrap()[0];

        let url_len = uri.as_bytes().len() as u64;
        let title_len = title.as_bytes().len() as u64;

        let entry = PageEntry {
            url_pos,
            title_pos,
            vector: embedding.as_slice().try_into().unwrap(),
            url_len,
            title_len,
        };
        let bytes: &[u8] = unsafe { any_as_u8_slice(&entry) };
        output_writer.write_all(bytes)?;

        url_writer.write_all(&uri.as_bytes())?;
        url_pos += url_len as u64;
        title_writer.write_all(&title.as_bytes())?;
        title_pos += title_len as u64;

        added += 1;
        let interval = 50;
        if added % interval == 0 {
            let duration = start.elapsed();
            speed = interval as f32 / duration.as_millis() as f32 * 1000.0;
            per_embedding = duration.as_millis() as f32 / interval as f32;
            start = Instant::now();
        }
        progress.set_message(format!("{} {:.0}/s {:.1} ms", added, speed, per_embedding));

        // 4 minutes
    }
    progress.finish();

    println!(
        "\nTotal Records of WARC file processed: {}, {} added",
        count, added
    );
    Ok(())
}
