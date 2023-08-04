use crate::search::page_source::PageSource;
use crate::search::vector::Embedding;
use crate::util::any_as_u8_slice;
use crate::util::default_progress_bar;
use rust_bert::pipelines::sentence_embeddings::SentenceEmbeddingsModel;
use std;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug)]
#[repr(C)]
pub struct PageEntry {
    pub url_pos: u64,
    pub title_pos: u64,
    pub vector: Embedding<f32>,
    pub url_len: u64,
    pub title_len: u64,
}

pub fn extract_records_and_add_to_index<T: Read>(
    input: &mut T,
    filename: &PathBuf,
    model: &SentenceEmbeddingsModel,
) -> io::Result<()> {
    let mut page_source = PageSource::read_warc_gz(input);

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

    let progress = default_progress_bar(36000);
    let mut start = Instant::now();
    let mut speed = 0.0f32;
    let mut per_embedding = 0.0f32;

    while let Some(record) = page_source.next()? {
        count += 1;
        progress.set_position(count);

        let embedding = &model.encode(&[record.combined]).unwrap()[0];

        let url_len = record.url.len() as u64;
        let title_len = record.title.len() as u64;

        let entry = PageEntry {
            url_pos,
            title_pos,
            vector: embedding.as_slice().try_into().unwrap(),
            url_len,
            title_len,
        };
        let bytes: &[u8] = unsafe { any_as_u8_slice(&entry) };
        output_writer.write_all(bytes)?;

        url_writer.write_all(&record.url.as_bytes())?;
        url_pos += url_len as u64;
        title_writer.write_all(&record.title.as_bytes())?;
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