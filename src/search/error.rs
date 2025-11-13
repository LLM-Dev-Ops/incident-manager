//! Error types for search operations

use crate::error::AppError;

/// Result type for search operations
pub type SearchResult<T> = std::result::Result<T, SearchError>;

/// Errors that can occur during search operations
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    /// Index initialization failed
    #[error("Index initialization failed: {0}")]
    IndexInitFailed(String),

    /// Index not found
    #[error("Index not found: {0}")]
    IndexNotFound(String),

    /// Query parsing failed
    #[error("Query parsing failed: {0}")]
    QueryParsingFailed(String),

    /// Search execution failed
    #[error("Search execution failed: {0}")]
    SearchFailed(String),

    /// Document indexing failed
    #[error("Document indexing failed: {0}")]
    IndexingFailed(String),

    /// Document deletion failed
    #[error("Document deletion failed: {0}")]
    DeletionFailed(String),

    /// Schema error
    #[error("Schema error: {0}")]
    SchemaError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Index corruption
    #[error("Index corruption detected: {0}")]
    IndexCorruption(String),

    /// Tantivy error
    #[error("Tantivy error: {0}")]
    TantivyError(String),
}

impl From<tantivy::TantivyError> for SearchError {
    fn from(err: tantivy::TantivyError) -> Self {
        SearchError::TantivyError(err.to_string())
    }
}

impl From<tantivy::query::QueryParserError> for SearchError {
    fn from(err: tantivy::query::QueryParserError) -> Self {
        SearchError::QueryParsingFailed(err.to_string())
    }
}

impl From<SearchError> for AppError {
    fn from(err: SearchError) -> Self {
        match err {
            SearchError::InvalidConfiguration(msg) => AppError::Configuration(msg),
            SearchError::IoError(err) => AppError::Internal(err.to_string()),
            _ => AppError::Internal(err.to_string()),
        }
    }
}
