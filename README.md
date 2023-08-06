# DawnSearch

[![Build Status](https://github.com/dawn-search/dawnsearch/workflows/Build/badge.svg?event=push)](https://github.com/dawn-search/dawnsearch/actions)
[![Crates.io](https://img.shields.io/crates/v/dawnsearch)](https://crates.io/crates/dawnsearch)
[![Crates.io](https://img.shields.io/crates/d/dawnsearch)](https://crates.io/crates/dawnsearch)
[![License](https://img.shields.io/crates/l/dawnsearch.svg)](LICENSE)

DawnSearch is an open source distributed web search engine that searches by meaning. It can index the [Common Crawl](https://commoncrawl.org/the-data/get-started/) data. It uses semantic search (searching on meaning), using [all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2). It uses [USearch](https://github.com/unum-cloud/usearch) for vector search. DawnSearch is written in [Rust](https://www.rust-lang.org/).

A public instance is available at [dawnsearch.org](https://dawnsearch.org).

### Project Status

DawnSearch currently functions as a distributed (semantic) vector search. When you start an instance, it will register with the tracker. The instance can then participate in the network by searching. Optionally, it can index the common crawl dataset and answer queries.

Main items still to do:

1. Better error handling. There still is a lot of .unwrap() in the code.
2. Robustness against malfunctioning or malicious instances.
3. Packet encryption to prevent eavesdropping.
4. Distribution of all the indexed pages to semantically close instances to increase search efficiency. Currently searches are sent to all instances.

# Quick start

This will build and run an 'access terminal' DawnSearch instance on a recent Ubuntu, without GPU acceleration. See [Modes](Modes.md) for examples of other configurations.

    sudo apt-get update && sudo apt-get install -y build-essential libssl-dev pkg-config python3-pip

    # Install rust if you don't have it already:
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

    pip3 install torch==2.0.0 --index-url https://download.pytorch.org/whl/cpu

Now we need to make sure the build system can find PyTorch. We search for the package:

    pip3 show torch

This prints the following:

    Name: torch
    Version: 2.0.0
    Summary: Tensors and Dynamic neural networks in Python with strong GPU acceleration
    Home-page: https://pytorch.org/
    Author: PyTorch Team
    Author-email: packages@pytorch.org
    License: BSD-3
    Location: /home/ubuntu/.local/lib/python3.10/site-packages
    Requires: filelock, jinja2, networkx, sympy, typing-extensions
    Required-by: 

Using the path from 'Location', put this in .bashrc. Note that you need to append '/torch'.

    export LIBTORCH=/home/ubuntu/.local/lib/python3.10/site-packages/torch
    export LD_LIBRARY_PATH=${LIBTORCH}/lib:$LD_LIBRARY_PATH

We can now load the new environment variables and build:

    source ~/.bashrc
    mv DawnSearch.toml.example DawnSearch.toml
    cargo run --release

Now, go to [http://localhost:8080](http://localhost:8080) to access your own DawnSearch instance. You will be able to perform searches, but you will not contribute to the network yet. Take a look at [Modes](Modes.md) to see how you can do so.

If you want to upgrade to GPU acceleration try this:

    pip3 remove torch
    pip3 install torch==2.0.0
    cargo clean
    cargo run --release

Alternatively, follow the steps as documented for the [tch](https://github.com/LaurentMazare/tch-rs) crate.

Note that on an M1/M2 Mac, 'cargo install' does NOT work. 'cargo build' does though!

Feel free to open an issue if you encounter problems!

# Configuration

You can configure DawnSearch through [DawnSearch.toml](DawnSearch.toml) or through environment variables like DAWNSEARCH_INDEX_CC.

# Contributing

Please open issues, or create pull requests! Please open an issue before you start working on a big enhancement or refactor.

# See also

- [How to build a Semantic Search Engine in Rust](https://sachaarbonel.medium.com/how-to-build-a-semantic-search-engine-in-rust-e96e6378cfd9) - Excellent tutorial on how to do semantic search with rust-bert.
