//! Common DTO types

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Generic paginated response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    /// Data items
    pub data: Vec<T>,
    /// Total count (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    /// Current page
    pub page: i64,
    /// Items per page
    pub limit: i64,
    /// Has more items
    pub has_more: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: i64, limit: i64, total: Option<i64>) -> Self {
        let has_more = total.map(|t| (page * limit) < t).unwrap_or(data.len() as i64 >= limit);
        Self {
            data,
            total,
            page,
            limit,
            has_more,
        }
    }
}

/// Pagination query parameters
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PaginationParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: i64,
    /// Items per page
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_page() -> i64 { 1 }
fn default_limit() -> i64 { 50 }

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            limit: 50,
        }
    }
}

impl PaginationParams {
    /// Get the offset for database queries
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit
    }

    /// Get the limit clamped to max
    pub fn clamped_limit(&self, max: i64) -> i64 {
        self.limit.min(max).max(1)
    }
}

/// Server time response (Binance-compatible)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerTimeResponse {
    /// Server time in milliseconds
    pub server_time: i64,
}

/// Generic success response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SuccessResponse {
    /// Success indicator
    pub success: bool,
    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl SuccessResponse {
    pub fn ok() -> Self {
        Self { success: true, message: None }
    }

    pub fn with_message(message: impl Into<String>) -> Self {
        Self { success: true, message: Some(message.into()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_offset() {
        let params = PaginationParams { page: 1, limit: 10 };
        assert_eq!(params.offset(), 0);

        let params = PaginationParams { page: 2, limit: 10 };
        assert_eq!(params.offset(), 10);

        let params = PaginationParams { page: 5, limit: 20 };
        assert_eq!(params.offset(), 80);
    }

    #[test]
    fn test_paginated_response() {
        let data = vec![1, 2, 3, 4, 5];
        let response = PaginatedResponse::new(data, 1, 10, Some(100));
        assert!(response.has_more);
        assert_eq!(response.total, Some(100));
    }
}
