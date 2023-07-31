# Profiling

    sudo apt-get install linux-tools-common linux-tools-generic linux-tools-`uname -r`

Run these as root (sudo -i):

    echo 0 > /proc/sys/kernel/kptr_restrict
    echo 0 > /proc/sys/kernel/perf_event_paranoid

Now we can start profiling:

    perf record -F99 --call-graph -- target/release/search_nsw ../wat2

    sudo apt install hotspot
    hotspot perf.data

# Godbolt Compiler Explorer

Make sure to specify the processor so you can see the vectorization: -O -C target-cpu=x86-64-v3
