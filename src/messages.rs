use crate::{page_source::ExtractedPage, search_provider::SearchResult, vector::Embedding};

pub enum SearchProviderMessage {
    TextSearch {
        otx: tokio::sync::oneshot::Sender<SearchResult>,
        query: String,
    },
    MoreLikeSearch {
        otx: tokio::sync::oneshot::Sender<SearchResult>,
        id: usize,
    },
    EmbeddingSearch {
        otx: tokio::sync::oneshot::Sender<SearchResult>,
        embedding: Box<Embedding<f32>>,
    },
    ExtractedPageMessage {
        page: ExtractedPage,
    },
    Save,
    Shutdown,
}
