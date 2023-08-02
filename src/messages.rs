use crate::{page_source::ExtractedPage, search_provider::SearchResult};

pub enum SearchProviderMessage {
    TextSearch {
        otx: tokio::sync::oneshot::Sender<SearchResult>,
        query: String,
    },
    MoreLikeSearch {
        otx: tokio::sync::oneshot::Sender<SearchResult>,
        id: usize,
    },
    ExtractedPageMessage {
        page: ExtractedPage,
    },
    Save,
    Shutdown,
}
