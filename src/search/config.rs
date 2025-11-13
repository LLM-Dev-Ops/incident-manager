//! Search configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Search service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Path to the search index directory
    pub index_path: PathBuf,

    /// Index writer heap size in bytes (default: 50MB)
    pub writer_heap_size: usize,

    /// Number of threads for indexing
    pub indexing_threads: usize,

    /// Enable real-time indexing
    pub realtime_indexing: bool,

    /// Commit interval in seconds (for real-time indexing)
    pub commit_interval_secs: u64,

    /// Maximum search results to return
    pub max_results: usize,

    /// Enable query suggestions
    pub enable_suggestions: bool,

    /// Enable faceted search
    pub enable_facets: bool,

    /// Enable highlighting
    pub enable_highlighting: bool,

    /// Cache size for search results
    pub cache_size: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            index_path: PathBuf::from("./data/search_index"),
            writer_heap_size: 50_000_000, // 50MB
            indexing_threads: 4,
            realtime_indexing: true,
            commit_interval_secs: 5,
            max_results: 1000,
            enable_suggestions: true,
            enable_facets: true,
            enable_highlighting: true,
            cache_size: 100,
        }
    }
}

/// Builder for SearchConfig
pub struct SearchConfigBuilder {
    config: SearchConfig,
}

impl SearchConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: SearchConfig::default(),
        }
    }

    pub fn index_path(mut self, path: PathBuf) -> Self {
        self.config.index_path = path;
        self
    }

    pub fn writer_heap_size(mut self, size: usize) -> Self {
        self.config.writer_heap_size = size;
        self
    }

    pub fn indexing_threads(mut self, threads: usize) -> Self {
        self.config.indexing_threads = threads;
        self
    }

    pub fn realtime_indexing(mut self, enabled: bool) -> Self {
        self.config.realtime_indexing = enabled;
        self
    }

    pub fn commit_interval_secs(mut self, secs: u64) -> Self {
        self.config.commit_interval_secs = secs;
        self
    }

    pub fn max_results(mut self, max: usize) -> Self {
        self.config.max_results = max;
        self
    }

    pub fn enable_suggestions(mut self, enabled: bool) -> Self {
        self.config.enable_suggestions = enabled;
        self
    }

    pub fn enable_facets(mut self, enabled: bool) -> Self {
        self.config.enable_facets = enabled;
        self
    }

    pub fn enable_highlighting(mut self, enabled: bool) -> Self {
        self.config.enable_highlighting = enabled;
        self
    }

    pub fn cache_size(mut self, size: usize) -> Self {
        self.config.cache_size = size;
        self
    }

    pub fn build(self) -> SearchConfig {
        self.config
    }
}

impl Default for SearchConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
