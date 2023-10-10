use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

/// Pagination parameters.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct PaginationParams {
    /// The 0-indexed page to fetch.
    page: Option<i64>,
    /// The number of elements per page.
    page_size: Option<i64>,
}

impl PaginationParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(0)
    }

    pub fn page_size(&self) -> i64 {
        self.page_size.unwrap_or(50)
    }

    pub fn limit(&self) -> i64 {
        self.page_size()
    }

    pub fn offset(&self) -> i64 {
        self.page() * self.page_size()
    }
}
