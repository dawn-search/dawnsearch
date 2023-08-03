use serde::{Deserialize, Serialize};

/**
 * With the IPv4 header being 20 bytes and the UDP header being 8 bytes, the payload of a UDP packet should be no larger than 1500 - 20 - 8 = 1472 bytes to avoid fragmentation.
 */

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum UdpMessage {
    /** Let other peers know we're here. */
    Announce { id: String },
    Search {
        // search_id: u64,
        /** Embedding quantized as i24, little endian. */
        #[serde(with = "serde_bytes")]
        embedding: Vec<u8>, // 1152
    },
    // /** Responder -> Searcher. The results we have available. */
    // SearchSummary { search_id: u64, distances: Vec<f32> },
    // /** Searcher -> Responder. Request to send all results below a certain distance. */
    // PageRequest { search_id: u64, max_distance: f32 },
    /** Responder -> Searcher. Information on a found page. */
    Page {
        // search_id: u64,
        distance: f32,
        url: String,   // 200?
        title: String, // 200?
        text: String,  // 500?
    },
}
