# DawnSearch

[![Build Status](https://github.com/dawn-search/dawnsearch/workflows/Build/badge.svg?event=push)](https://github.com/dawn-search/dawnsearch/actions)
[![Crates.io](https://img.shields.io/crates/v/dawnsearch)](https://crates.io/crates/dawnsearch)
[![Crates.io](https://img.shields.io/crates/d/dawnsearch)](https://crates.io/crates/dawnsearch)
[![License](https://img.shields.io/crates/l/dawnsearch.svg)](LICENSE)

DawnSearch is an open source distributed web search engine that searches by meaning. It uses semantic search (searching on meaning), using [all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2). It uses [USearch](https://github.com/unum-cloud/usearch) for vector search. It can index the [Common Crawl](https://commoncrawl.org/the-data/get-started/) data. DawnSearch is written in [Rust](https://www.rust-lang.org/).

A public instance is available at [dawnsearch.org](https://dawnsearch.org).

## Project Status

DawnSearch currently functions as a distributed (semantic) vector search. When you start an instance, it will register with the tracker. The instance can then participate in the network by searching. Optionally, it can index the common crawl dataset and answer queries.

Main items still to do:

1. Better error handling. There still is a lot of .unwrap() in the code.
2. Robustness against malfunctioning or malicious instances.
3. Packet encryption.
4. Increase search efficiency by distributing indexed pages to instances that are semantically close to the content.

## Help needed!

DawnSearch is looking for:

1. People to use the search on [dawnsearch.org](https://dawnsearch.org) and give feedback on the useability and quality of results.
2. People with Rust experience to take a look at the codebase and give tips on making it easier to read and more ideomatic Rust.
3. A UI/UX designer to create designs for the main and search results pages.
4. Rust developers who can tackle some of the problems mentioned under 'Project Status'
5. People who want to run their own instance.

Please open issues for any questions or feedback. If you want to contribute something big, like a feature or a refactor, open an issue before you start so you don't do duplicate work!

## Quick start

This will build and run an 'access terminal' DawnSearch instance on a recent Ubuntu, without GPU acceleration. See [Modes](Modes.md) for examples of other configurations.

    sudo apt-get update && sudo apt-get install -y build-essential pkg-config

    # Install rust if you don't have it already:
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

    mv DawnSearch.toml.example DawnSearch.toml
    RUSTFLAGS='-C target-cpu=native'  cargo run --release

Now, go to [http://localhost:8080](http://localhost:8080) to access your own DawnSearch instance. You will be able to perform searches, but you will not contribute to the network yet. Take a look at [Modes](Modes.md) to see how you can do so.

If you want to upgrade to GPU acceleration try this. You need to have CUDA installed:

    RUSTFLAGS='-C target-cpu=native'  cargo run --release --features cuda

Note that on an M1/M2 Mac, 'cargo install' does NOT work. 'cargo build' does though!

Feel free to open an issue if you encounter problems!

## Configuration

You can configure DawnSearch through [DawnSearch.toml](DawnSearch.toml) or through environment variables like DAWNSEARCH_INDEX_CC.

## Documentation

Work in progress!

- [DawnSearch Architecture](doc/architecture.md)
- [Additional information on buildling DawnSearch](doc/build.md)
- [Data](doc/data.md) - Location of the data stored by DawnSearch.
- [DawnSearch Modes](doc/modes.md) - The different ways you can run DawnSearch.
- [Optmizing](doc/optimizing.md) - profiling and optimizing.

## See also

- [How to build a Semantic Search Engine in Rust](https://sachaarbonel.medium.com/how-to-build-a-semantic-search-engine-in-rust-e96e6378cfd9) - Excellent tutorial on how to do semantic search with rust-bert.
