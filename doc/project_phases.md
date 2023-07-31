# Common

For now, only the English part of the internet will be indexed, as this is the language with the most [total speakers](https://en.wikipedia.org/wiki/List_of_languages_by_total_number_of_speakers). Arecbino will also only index text, from html pages (text/html).

# Phase 1 - Centralized

Server-side search, with precomputed embeddings. This will scale op to 1000 CC blocks, and five million documents searched. 

# Phase 2 - Collaborative

'search@home', a distributed collaborative effort to embed and search the entire common crawl (CC) corpus. The nodes will get assigned one of the 80.000 1.2 GB CC blocks, and download and embed these. The found embeddings are sent to the central server. The central server distributes these to the nodes in order to farm out searches to them.

# Phase 3 - Decentralized

Nodes will start connecting directly to each other, using a [Kademlia](https://en.wikipedia.org/wiki/Kademlia) like network. 