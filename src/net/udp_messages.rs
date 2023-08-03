use serde::{Deserialize, Serialize};

/**
 * With the IPv4 header being 20 bytes and the UDP header being 8 bytes, the payload of a UDP packet should be no larger than 1500 - 20 - 8 = 1472 bytes to avoid fragmentation.
 */

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum UdpMessage {
    #[serde(rename = "s")]
    Search {
        // search_id: u64,
        /** Embedding quantized as i24, little endian. */
        #[serde(rename = "e")]
        #[serde(with = "serde_bytes")]
        embedding: Vec<u8>, // 1152
    },
    // /** Responder -> Searcher. The results we have available. */
    // SearchSummary { search_id: u64, distances: Vec<f32> },
    // /** Searcher -> Responder. Request to send all results below a certain distance. */
    // PageRequest { search_id: u64, max_distance: f32 },
    /** Responder -> Searcher. Information on a found page. */
    #[serde(rename = "pg")]
    Page {
        // search_id: u64,
        #[serde(rename = "d")]
        distance: f32,
        #[serde(rename = "u")]
        url: String, // 200?
        #[serde(rename = "t")]
        title: String, // 200?
        #[serde(rename = "x")]
        text: String, // 500?
    },
    ////////////////////
    // Tracker messages
    /** Let other peers know we're here. */
    #[serde(rename = "a")]
    Announce { id: String },
    #[serde(rename = "p")]
    Peers {
        #[serde(rename = "p")]
        peers: Vec<PeerInfo>,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct PeerInfo {
    pub id: String,
    pub addr: String, // TODO: replace by binary value.
    pub last_seen: u64,
}
