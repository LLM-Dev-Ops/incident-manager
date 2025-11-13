//! Enterprise-grade full-text search system powered by Tantivy
//!
//! This module provides comprehensive search capabilities for incident management,
//! including:
//!
//! - **Full-Text Search**: Search across incident titles, descriptions, and metadata
//! - **Faceted Search**: Filter by severity, type, status, source, etc.
//! - **Real-Time Indexing**: Automatic index updates on incident changes
//! - **Advanced Queries**: Boolean queries, phrase matching, fuzzy search
//! - **Ranking & Scoring**: BM25 ranking with custom boosting
//! - **Aggregations**: Count by severity, type, status, etc.
//! - **Highlighting**: Search term highlighting in results
//!
//! # Architecture
//!
//! The search system uses Tantivy, a fast full-text search engine library:
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │           Search Service API                     │
//! ├─────────────────────────────────────────────────┤
//! │  - search()        - search_with_facets()       │
//! │  - suggest()       - aggregate()                │
//! │  - index_incident() - delete_incident()         │
//! └─────────────────────────────────────────────────┘
//!                      │
//!                      ▼
//! ┌─────────────────────────────────────────────────┐
//! │           Index Manager                          │
//! ├─────────────────────────────────────────────────┤
//! │  - Schema Management                             │
//! │  - Index Writer/Reader Pool                      │
//! │  - Transaction Management                        │
//! └─────────────────────────────────────────────────┘
//!                      │
//!                      ▼
//! ┌─────────────────────────────────────────────────┐
//! │              Tantivy Index                       │
//! ├─────────────────────────────────────────────────┤
//! │  - Inverted Index (title, description)          │
//! │  - Fast Fields (severity, type, timestamps)     │
//! │  - Doc Store (full documents)                   │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use llm_incident_manager::search::{SearchService, SearchQuery, SearchConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = SearchConfig::default();
//!     let search = SearchService::new(config).await?;
//!
//!     // Simple search
//!     let query = SearchQuery::new("database error")
//!         .with_severity(vec!["P0", "P1"])
//!         .with_limit(20);
//!
//!     let results = search.search(&query).await?;
//!     println!("Found {} incidents", results.total_hits);
//!
//!     Ok(())
//! }
//! ```

mod config;
mod document;
mod error;
mod index;
mod query;
mod service;

pub use config::{SearchConfig, SearchConfigBuilder};
pub use document::{IncidentDocument, SearchDocument};
pub use error::{SearchError, SearchResult};
pub use index::{IndexManager, IndexStats};
pub use query::{
    Aggregation, FacetCount, QueryBuilder, SearchFilter, SearchQuery, SearchSort, SortOrder,
};
pub use service::{SearchHit, SearchResponse, SearchService, SearchSuggestion};
