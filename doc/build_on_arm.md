This document describes how to compile on arm64 linux, for example the AWS Graviton instances.
4GB of ram needed, for torch-sys, so on AWS that is a t4g.medium.

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

Log out & log in

    sudo apt update
    sudo apt install build-essential pkg-config pip3

    pip3 install torch==2.0.0

Now, there are two ways to continue. Note that if you run into problems, and fiddle with things, you need to run 'cargo clean'. The system will get very confused otherwise and you will get linker errors.

## First method

Add the following to your .bashrc

    export LIBTORCH_USE_PYTORCH=1

## Second method

Find the install location

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

Now run

    source ~/.bashrc
    cargo build --release

# On Zram

You may be tempted to use zram on a low ram machine. This sort-of works for building, though things will be really slow. However, don't use this for running DawnSearch, as the part of the index that stays in memory is not very compressible, and this will add latency to searches.

# Nginx

    sudo apt install nginx

Follow these steps to set up nginx as reverse proxy server: https://www.digitalocean.com/community/tutorials/how-to-configure-nginx-as-a-reverse-proxy-on-ubuntu-22-04

Follow these steps to enable letsencrypt: https://www.nginx.com/blog/using-free-ssltls-certificates-from-lets-encrypt-with-nginx/

Follow these steps on how to run as a service.

Example file:

    [Unit]
    Description=DawnSearch
    After=network.target

    [Service]
    Type=simple
    ExecStart=/home/ubuntu/dawnsearch2/target/release/dawnsearch
    WorkingDirectory=/home/ubuntu/dawnsearch2
    Restart=always
    RestartSec=5
    StandardOutput=syslog
    StandardError=syslog
    SyslogIdentifier=%n
    Environment="LIBTORCH=/home/ubuntu/.local/lib/python3.10/site-packages/torch"
    Environment="LD_LIBRARY_PATH=/home/ubuntu/.local/lib/python3.10/site-packages/torch/lib"

    [Install]
    WantedBy=multi-user.target








