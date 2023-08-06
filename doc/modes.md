# Modes

You can run DawnSearch in a number of different modes, using different settings from the config file. This page describes some of them.

## The access terminal

Use this if you want to search all the data in DawnSearch, but you don't want to index any data or respond to any queries.

    web = true
    udp = true
    index_cc = false
    accept_insert = false

# The indexer

When configured like this, DawnSearch will index pages and make them available for searching on the network.

    web = false
    udp = true
    index_cc = true
    accept_insert = false

# The storage

In this case, the instance will accept inserts from the network, and allow other people to search for them.

    web = false
    udp = true
    index_cc = false
    accept_insert = true

