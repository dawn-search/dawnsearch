# DawnSearch

[![Build Status](https://github.com/dawn-search/dawnsearch/workflows/Build/badge.svg?event=push)](https://github.com/dawn-search/dawnsearch/actions)
![Crates.io](https://img.shields.io/crates/v/dawnsearch)
![License](https://img.shields.io/crates/l/dawnsearch.svg)

DawnSearch is an open source distributed web search engine that searches by meaning. It can index the [Common Crawl](https://commoncrawl.org/the-data/get-started/) data. It uses semantic search (searching on meaning), using [all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2). It uses [USearch](https://github.com/unum-cloud/usearch) for vector search. DawnSearch is written in [Rust](https://www.rust-lang.org/). DawnSearch is licensed [AGPLv3.0+](LICENSE).

A public instance is available at [dawnsearch.org](https://dawnsearch.or).

# Quick start

These instructions assume you're running a recent Ubuntu.

Install Rust (if you don't have it already):

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

Install the required packages.

    sudo apt update
    sudo apt install build-essential libssl-dev pkg-config

Next, install libtorch.

* To use DawnSearch with your CPU only, use pip to install pytorch and add "export LIBTORCH_USE_PYTORCH=1" to your .bashrc.
* If you want CUDA support, follow the manual(!) installation steps from https://github.com/guillaume-be/rust-bert. If you don't use the manual steps CUDA may not work.

Feel free to open an issue if you encounter problems building DawnSearch!

# Contributing

Please open issues, or create pull requests. Note that DawnSearch is licensed AGPLv3.0+ or later, which is slightly unusual for a Rust project.

# See also

- [How to build a Semantic Search Engine in Rust](https://sachaarbonel.medium.com/how-to-build-a-semantic-search-engine-in-rust-e96e6378cfd9) - Excellent tutorial on how to do semantic search with rust-bert.
