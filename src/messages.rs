use crate::{page_source::ExtractedPage, search_provider::SearchResult};

pub enum SearchProviderMessage {
    SearchRequestMessage {
        otx: tokio::sync::oneshot::Sender<SearchResult>,
        query: String,
    },
    ExtractedPageMessage {
        page: ExtractedPage,
    },
}
