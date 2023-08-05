# DawnSearch

[![Build Status](https://github.com/dawn-search/dawnsearch/workflows/Build/badge.svg?event=push)](https://github.com/dawn-search/dawnsearch/actions)
[![Crates.io](https://img.shields.io/crates/v/dawnsearch)](https://crates.io/crates/dawnsearch)
[![License](https://img.shields.io/crates/l/dawnsearch.svg)](LICENSE)

DawnSearch is an open source distributed web search engine that searches by meaning. It can index the [Common Crawl](https://commoncrawl.org/the-data/get-started/) data. It uses semantic search (searching on meaning), using [all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2). It uses [USearch](https://github.com/unum-cloud/usearch) for vector search. DawnSearch is written in [Rust](https://www.rust-lang.org/). DawnSearch is licensed [AGPLv3.0+](LICENSE).

A public instance is available at [dawnsearch.org](https://dawnsearch.or).

# Quick start

This will build and run DawnSearch on a recent Ubuntu, without GPU acceleration.

    sudo apt-get update && sudo apt-get install -y build-essential libssl-dev pkg-config python3-pip

    # Install rust if you don't have it already:
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

    pip3 install torch==2.0.0 --index-url https://download.pytorch.org/whl/cpu
    export LIBTORCH_USE_PYTORCH=1 # You probably want to add this to your .bashrc at some point
    cargo run --release

If you want to upgrade to GPU acceleration do try this:

    pip3 install torch==2.0.0
    cargo clean
    cargo run --release

Alternatively, follow the steps as documented for the [tch](https://github.com/LaurentMazare/tch-rs) crate.

Feel free to open an issue if you encounter problems!

# Configuration

You can configure DawnSearch through [DawnSearch.toml](DawnSearch.toml) or through environment variables like DAWNSEARCH_INDEX_CC.

# Contributing

Please open issues, or create pull requests. Note that DawnSearch is licensed AGPLv3.0+ or later, which is slightly unusual for a Rust project.

# See also

- [How to build a Semantic Search Engine in Rust](https://sachaarbonel.medium.com/how-to-build-a-semantic-search-engine-in-rust-e96e6378cfd9) - Excellent tutorial on how to do semantic search with rust-bert.
