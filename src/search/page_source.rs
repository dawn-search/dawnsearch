use std;
use std::io;
use std::io::BufRead;

use flate2::read::MultiGzDecoder;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::RcDom;
use std::io::Read;
use url::Url;
use whichlang::detect_language;
use whichlang::Lang;

use crate::index::extract::extract;
use crate::index::extract::extract_text;
use crate::util::slice_up_to;

struct RecordOwned {
    uri: String,
    warc_type: String,
    payload_type: String,
    body: String,
}

#[derive(Clone)]
pub struct ExtractedPage {
    pub url: String,
    pub title: String,
    pub text: String,
    pub combined: String,
}

pub struct PageSource<T: Read> {
    reader: io::BufReader<MultiGzDecoder<T>>,
}

impl<T: Read> PageSource<T> {
    pub fn read_warc_gz(input: T) -> PageSource<T> {
        const PER_THREAD_BUF_SIZE: usize = 16 * 1024 * 1024;

        let reader = io::BufReader::with_capacity(PER_THREAD_BUF_SIZE, MultiGzDecoder::new(input));
        PageSource { reader }
    }
    pub fn next(&mut self) -> Result<Option<ExtractedPage>, io::Error> {
        while let Some(record) = read_record(&mut self.reader)? {
            if record.warc_type != "conversion" && record.warc_type != "response" {
                continue;
            }
            if record.payload_type != "text/html" {
                continue;
            }

            let uri = record.uri;
            if uri.contains("?") || uri.contains("#") {
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

            if clean.len() < 400 {
                continue;
            }

            let title = slice_up_to(&title, 200);
            let clean = slice_up_to(&clean, 2048);

            let mut combined = title.to_string();
            combined.push(' ');
            combined.push_str(&clean);

            let lang = detect_language(&combined);
            if lang != Lang::Eng {
                continue;
            }
            return Ok(Some(ExtractedPage {
                url: url.to_string(),
                title: title.to_string(),
                text: clean.to_string(),
                combined,
            }));
        }
        Ok(None)
    }
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
