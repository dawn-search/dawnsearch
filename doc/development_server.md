# Devlopment server

When working on DawnSearch, it can be useful to set up a server where you can test local versions on. This page describes how to do this.

Create a new remote on you local machine, let's call this 'test'. Note that this should be a different directory from your real checkout!

    git remote add test <server>:<path>

Now we can push our changes there:

    git push test

On your test server you then run.

    git remote add local <path> # One time
    git pull local main

Now we have the changes and can build & update

    cargo build --release
    sudo systemctl restart dawnsearch.service

If you also run a tracker, reload like this

    sudo systemctl restart dawntrack.service