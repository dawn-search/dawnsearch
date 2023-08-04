# Searching the Network

- DawnSearch first calculates the embedding of the query string, and quantizes it.
- The query is sent to the closest node we know of, with a cutoff distance and a max number of results.
- If the node has these results, it sends them back. Otherwise it sends a list of nodes that are closer to the search.
- We can then contact these nodes.
- Whe we get results, we know how far the worst of these is from the division line.
- We query the other half of the division lines recursively until we find one that is closer.

# Inserting into the network

- We embed and quantize.
- We search for the closest node(s).
- We insert to these nodes.

# Routing table

- Just like Kadmilia uses xor, we use a kd tree.
- The first bucket of our routing table contains the 'other half'.
- The rest of the buckets contain peers that are in our 'half'.

# Hierarchical Navigable Small World

DawnSearch will effectively be a two layer HNSW search. The first one will find the node that contains that data, then the node will use HNSW itself to find the data you need.

https://github.com/rust-cv/hnsw