use crate::{page_source::ExtractedPage, search_provider::SearchResult};

pub enum SearchProviderMessage {
    SearchRequestMessage {
        otx: tokio::sync::oneshot::Sender<SearchRequestResponse>,
        query: String,
    },
    ExtractedPageMessage {
        page: ExtractedPage,
    },
}

#[derive(Debug)]
pub struct SearchRequestResponse {
    pub results: Vec<SearchResult>,
}
