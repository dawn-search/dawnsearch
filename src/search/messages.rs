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

use super::page_source::ExtractedPage;
use super::search_provider::{SearchResult, SearchStats};

#[derive(Debug)]
pub enum SearchProviderMessage {
    TextSearch {
        otx: tokio::sync::oneshot::Sender<SearchResult>,
        query: String,
    },
    MoreLikeSearch {
        otx: tokio::sync::oneshot::Sender<SearchResult>,
        instance_id: String,
        page_id: usize,
    },
    EmbeddingSearch {
        otx: tokio::sync::oneshot::Sender<SearchResult>,
        embedding: Vec<f32>,
        search_remote: bool,
    },
    ExtractedPageMessage {
        page: ExtractedPage,
        from_network: bool,
    },
    GetEmbedding {
        page_id: usize,
        otx: tokio::sync::oneshot::Sender<Vec<f32>>,
    },
    Stats {
        otx: tokio::sync::oneshot::Sender<SearchStats>,
    },
    Save,
    Shutdown,
}
