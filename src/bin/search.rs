use std::fs::File;
use std::io::{self, BufRead};
use std::str;
use std::time::Instant;
use std::{self};

use memmap2::{Mmap, MmapOptions};
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};
use std::env;

use arecibo::vector::{distance, EM_LEN};
use arecibo::warc::PageEntry;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let warc_dir = &args[1];

    let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
        .create_model()?;

    //////////////

    let mut emb_files: Vec<Mmap> = Vec::new();
    let mut title_files: Vec<Mmap> = Vec::new();
    let mut url_files: Vec<Mmap> = Vec::new();

    let mut numfiles = 0;
    for path in std::fs::read_dir(warc_dir).unwrap() {
        numfiles += 1;
        let filename = path.unwrap().path();
        let mut url_filename = filename.clone();
        let mut title_filename = filename.clone();
        let s = filename.to_string_lossy();
        if !s.ends_with(".warc.emb") {
            continue;
        }
        eprintln!("{}\t{}", numfiles, s);

        // Now we read back our data from the files.
        let f = File::open(filename)?;
        let mmap = unsafe { MmapOptions::new().map(&f)? };
        emb_files.push(mmap);

        url_filename.set_extension("url");
        let f = File::open(url_filename)?;
        let mmap = unsafe { MmapOptions::new().map(&f)? };
        url_files.push(mmap);

        title_filename.set_extension("title");
        let f = File::open(title_filename)?;
        let mmap = unsafe { MmapOptions::new().map(&f)? };
        title_files.push(mmap);
    }

    struct ScoredBook {
        score: f32,
        file: usize,
        index: usize,
    }

    let stdin = io::stdin();
    eprint!("> ");
    for q in stdin.lock().lines() {
        println!("");
        let query = q.unwrap();

        let start = Instant::now();

        let q = &model.encode(&[query]).unwrap()[0];
        let query_embedding: &[f32; EM_LEN] = q.as_slice().try_into().unwrap();

        let mut results: Vec<ScoredBook> = Vec::new();

        let size = std::mem::size_of::<PageEntry>();
        let mut searched_pages_count = 0;
        for (i, mmap) in emb_files.iter().enumerate() {
            let mut pos = 0;
            while pos + size < mmap.len() {
                searched_pages_count += 1;
                let p: &PageEntry = unsafe { &*mmap[pos..pos + size].as_ptr().cast() };

                let score = distance(&p.vector, &query_embedding);
                if results.len() < 10 {
                    results.push(ScoredBook {
                        file: i,
                        score,
                        index: pos,
                    });
                    continue;
                }
                if score < results[9].score {
                    results[9] = ScoredBook {
                        file: i,
                        score,
                        index: pos,
                    };
                    results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
                }

                pos += size;
            }
        }

        //////////////

        for r in results {
            let p: &PageEntry =
                unsafe { &*emb_files[r.file][r.index..r.index + size].as_ptr().cast() };
            let url: &[u8] =
                &url_files[r.file][p.url_pos as usize..(p.url_pos + p.url_len as u64) as usize];
            let title: &[u8] = &title_files[r.file]
                [p.title_pos as usize..(p.title_pos + p.title_len as u64) as usize];
            println!(
                "{}: {} - {}",
                r.score,
                unsafe { str::from_utf8_unchecked(title) },
                unsafe { str::from_utf8_unchecked(url) },
            );
        }
        let duration = start.elapsed();
        let fraction = searched_pages_count as f32 / (80000.0 * 7000.0);
        println!("");
        println!(
            "Searched {} pages in {:.1} ms ({:.2}% of the common crawl database)",
            searched_pages_count,
            duration.as_millis(),
            fraction * 100.0
        );
        println!("");
        eprint!("> ");
    }

    Ok(())
}
