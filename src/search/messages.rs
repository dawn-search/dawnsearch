use super::page_source::ExtractedPage;
use super::search_provider::SearchResult;

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
        embedding: Vec<f32>,
    },
    ExtractedPageMessage {
        page: ExtractedPage,
    },
    Save,
    Shutdown,
}
