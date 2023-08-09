# Cuda

IMPORTANT: before you start check if your device supports CUDA capability 7.0 or higher. Candle requires this as of August 2023.

To run with GPU acceleration, install CUDA using the instructions from https://developer.nvidia.com/cuda-downloads.

Add the following to your .bashrc

    export CUDA_HOME=/usr/local/cuda
    export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:/usr/local/cuda/lib64:/usr/local/cuda/extras/CUPTI/lib64
    export PATH=$PATH:$CUDA_HOME/bin

Then, restart your terminal or source it.

Check supported capability

    nvidia-smi --query-gpu=compute_cap --format=csv

This should be 7.0 or higher.

Now you can build DawnSearch with:

    RUSTFLAGS='-C target-cpu=native' cargo build --release --features cuda


## Troubleshooting

See if CUDA works at all by checking this

https://askubuntu.com/a/1215237