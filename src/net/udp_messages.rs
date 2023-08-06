/*
   Copyright 2023 Krol Inventions B.V.

   This file is part of DawnSearch.

   DawnSearch is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   DawnSearch is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with DawnSearch.  If not, see <https://www.gnu.org/licenses/>.
*/

use serde::{Deserialize, Serialize};

/**
 * With the IPv4 header being 20 bytes and the UDP header being 8 bytes, the payload of a UDP packet should be no larger than 1500 - 20 - 8 = 1472 bytes to avoid fragmentation.
 */

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum UdpMessage {
    #[serde(rename = "s")]
    Search {
        #[serde(rename = "si")]
        search_id: u64,
        /** Do not return pages with a distance bigger than this. */
        #[serde(rename = "dl")]
        distance_limit: Option<f32>,
        /** Embedding quantized as i24, little endian. */
        #[serde(rename = "em")]
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
        #[serde(rename = "si")]
        search_id: u64,
        #[serde(rename = "di")]
        distance: f32,
        #[serde(rename = "ur")]
        url: String, // 200?
        #[serde(rename = "ti")]
        title: String, // 200?
        #[serde(rename = "te")]
        text: String, // 500?

        #[serde(rename = "ii")]
        instance_id: String,
    },
    Insert {
        #[serde(rename = "us")]
        #[serde(with = "serde_bytes")]
        url_smaz: Vec<u8>,
        #[serde(rename = "ts")]
        #[serde(with = "serde_bytes")]
        title_smaz: Vec<u8>,
        #[serde(rename = "xs")]
        #[serde(with = "serde_bytes")]
        text_smaz: Vec<u8>,
    },
    ////////////////////
    // Tracker messages
    /** Let other peers know we're here. */
    #[serde(rename = "a")]
    Announce {
        #[serde(rename = "ii")]
        instance_id: String,
        #[serde(rename = "ai")]
        accept_insert: bool,
        #[serde(rename = "pi")]
        pages_indexed: usize,
    },
    #[serde(rename = "p")]
    Peers {
        #[serde(rename = "pe")]
        peers: Vec<PeerInfo>,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct PeerInfo {
    #[serde(rename = "ii")]
    pub instance_id: String,
    #[serde(rename = "a")]
    pub addr: String, // TODO: replace by binary value.
    #[serde(rename = "ls")]
    pub last_seen: u64,
    #[serde(rename = "ai")]
    pub accept_insert: bool,
    #[serde(rename = "pi")]
    pub pages_indexed: usize,
}
