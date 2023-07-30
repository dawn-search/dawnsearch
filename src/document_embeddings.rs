use std::fs::File;

use memmap2::{Mmap, MmapOptions};

use crate::warc::PageEntry;

/**
 * Memory mapped list of embeddings.
 */
pub struct DocumentEmbeddings {
    emb_files: Vec<Mmap>,
    title_files: Vec<Mmap>,
    url_files: Vec<Mmap>,
}

impl DocumentEmbeddings {
    pub fn load(warc_dir: &str) -> anyhow::Result<DocumentEmbeddings> {
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
        Ok(DocumentEmbeddings {
            emb_files,
            title_files,
            url_files,
        })
    }

    pub fn files(&self) -> usize {
        self.emb_files.len()
    }

    pub fn entries(&self, page: usize) -> usize {
        self.emb_files[page].len() / std::mem::size_of::<PageEntry>()
    }

    pub fn entry(&self, file: usize, entry: usize) -> &PageEntry {
        let size = std::mem::size_of::<PageEntry>();
        return unsafe {
            &*self.emb_files[file][entry * size..entry * size + size]
                .as_ptr()
                .cast()
        };
    }

    pub fn url(&self, file: usize, entry: usize) -> &[u8] {
        let p = self.entry(file, entry);
        &self.url_files[file][p.url_pos as usize..(p.url_pos + p.url_len as u64) as usize]
    }

    pub fn title(&self, file: usize, entry: usize) -> &[u8] {
        let p = self.entry(file, entry);
        &self.title_files[file][p.title_pos as usize..(p.title_pos + p.title_len as u64) as usize]
    }
}
