# DawnSearch Search

DawnSearch is a search engine you can run on your own computer. It can index [WARC](https://en.wikipedia.org/wiki/WARC_(file_format)) files, for example from [Common Crawl](https://commoncrawl.org/the-data/get-started/). It uses semantic search, using [all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2). HTML is cleaned using [readability-rs](https://github.com/kumabook/readability) in order to only index the main content of the page.

The ultimate goal of DawnSearch is to provide a fully open source and noncommercial alternative to Google.

# Quick start

These instructions assume you're running a recent Ubuntu.

Install Rust (if you don't have it already):

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

Install the required packages.

    sudo apt update
    sudo apt install build-essential libssl-dev pkg-config

Next, install libtorch. Follow the manual(!) installation steps from https://github.com/guillaume-be/rust-bert. If you don't use the manual steps CUDA may not work. For mac (M1) use pip to install pytorch and use 'pip3 show' to find the path.

If you want to run DawnSearch on AWS EC2, follow the instructions from https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/install-nvidia-driver.html to install the required drivers.

You can now download WARC files into a directory, for example into '../wat'. There is no need to extract them, the indexer will work on gz files.

To index:

    cargo run --release --bin index ../wat

To search (interactive):

    cargo run --release --bin search ../wat

Feel free to open an issue if you encounter problems building DawnSearch!

# Postgres setup

    sudo apt install postgres
    sudo pg_ctlcluster 12 main start
    sudo -u postgres psql

Run these commands:

    create database dawnsearch;
    create user dawnsearch with encrypted password 'dawnsearch';
    grant all privileges on database dawnsearch to dawnsearch;

# FAQ
## Why is it called 'DawnSearch'?

The [DawnSearch Telescope](https://en.wikipedia.org/wiki/DawnSearch_Telescope) was the world's largest telescope for more than 50 years.
It also appeared in the James Bond movie [GoldenEye](https://en.wikipedia.org/wiki/GoldenEye) (1995), which is, to this day
the favorite bond movie of the author.

## Why Rust?

Rust is not the best platform to experiment with inference, though there are some
nice libraries. The big advantage of Rust is that while it takes ten times as long
to build something, it is then twice as fast, and uses much less memory.
This makes it much easier to host on left-over servers.

## How fast is it?

Indexing takes 2-4 minutes per WAT file. And Common Crawl has 80.000 of these. But DawnSearch is fun and useful with only 10 WAT files (12 GB) of data.

## How big is the index?

The index is about 12 MB per 1200 MB WAT file, so about 100x smaller. This means a full index of Common Crawl would be about 1TB, which is not too much data.

## Is this full-text search?

DawnSearch currently only indexes the first 2kb of text. This is much faster and also seems to improve precision.

# See also

- [How to build a Semantic Search Engine in Rust](https://sachaarbonel.medium.com/how-to-build-a-semantic-search-engine-in-rust-e96e6378cfd9) - Excellent tutorial on how to do semantic search with rust-bert.
- [Konnu](https://gitlab.com/shadowislord/konnu) - Early stage project for a peer to peer full-text search engine written in Rust. Currently (2023) only runs simulated.
- [tantivy_warc_indexer](https://github.com/ahcm/tantivy_warc_indexer) - Good reference on how to read WET files.