//! Search index management

use crate::search::config::SearchConfig;
use crate::search::document::{build_incident_schema, IncidentDocument, SearchDocument};
use crate::search::error::{SearchError, SearchResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tantivy::collector::Count;
use tantivy::schema::Schema;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy};
use tokio::sync::RwLock;

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Total number of documents in the index
    pub total_documents: u64,

    /// Index size in bytes
    pub index_size_bytes: u64,

    /// Number of segments
    pub num_segments: usize,

    /// Last commit timestamp
    pub last_commit: Option<chrono::DateTime<chrono::Utc>>,
}

/// Manages the Tantivy search index
pub struct IndexManager {
    /// The Tantivy index
    index: Index,

    /// The schema
    schema: Schema,

    /// Index writer (wrapped in RwLock for thread-safety)
    writer: Arc<RwLock<IndexWriter>>,

    /// Index reader
    reader: IndexReader,

    /// Configuration
    config: SearchConfig,
}

impl IndexManager {
    /// Create a new IndexManager
    pub async fn new(config: SearchConfig) -> SearchResult<Self> {
        // Create index directory if it doesn't exist
        std::fs::create_dir_all(&config.index_path).map_err(|e| {
            SearchError::IndexInitFailed(format!("Failed to create index directory: {}", e))
        })?;

        // Build schema
        let schema = build_incident_schema();

        // Open or create index
        let index = if Self::index_exists(&config.index_path) {
            Index::open_in_dir(&config.index_path).map_err(|e| {
                SearchError::IndexInitFailed(format!("Failed to open existing index: {}", e))
            })?
        } else {
            Index::create_in_dir(&config.index_path, schema.clone()).map_err(|e| {
                SearchError::IndexInitFailed(format!("Failed to create new index: {}", e))
            })?
        };

        // Create index writer
        let writer = index
            .writer(config.writer_heap_size)
            .map_err(|e| SearchError::IndexInitFailed(format!("Failed to create writer: {}", e)))?;

        // Create index reader with reload policy
        let reader = index
            .reader_builder()
            .reload_policy(if config.realtime_indexing {
                ReloadPolicy::OnCommitWithDelay
            } else {
                ReloadPolicy::Manual
            })
            .try_into()
            .map_err(|e| SearchError::IndexInitFailed(format!("Failed to create reader: {}", e)))?;

        Ok(Self {
            index,
            schema,
            writer: Arc::new(RwLock::new(writer)),
            reader,
            config,
        })
    }

    /// Check if an index exists at the given path
    fn index_exists(path: &Path) -> bool {
        path.join("meta.json").exists()
    }

    /// Get the schema
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Get the index
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Get the reader
    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }

    /// Index a single incident document
    pub async fn index_document(&self, document: &IncidentDocument) -> SearchResult<()> {
        let tantivy_doc = document.to_tantivy_doc(&self.schema);

        let mut writer = self.writer.write().await;

        // Delete existing document with same ID first
        if let Ok(id_field) = self.schema.get_field("id") {
            let term = tantivy::Term::from_field_text(id_field, &document.document_id());
            writer.delete_term(term);
        }

        // Add the document
        writer
            .add_document(tantivy_doc)
            .map_err(|e| SearchError::IndexingFailed(format!("Failed to add document: {}", e)))?;

        // Commit if real-time indexing is enabled
        if self.config.realtime_indexing {
            writer.commit().map_err(|e| {
                SearchError::IndexingFailed(format!("Failed to commit document: {}", e))
            })?;
        }

        Ok(())
    }

    /// Index multiple incident documents
    pub async fn index_documents(&self, documents: &[IncidentDocument]) -> SearchResult<usize> {
        let mut writer = self.writer.write().await;
        let mut indexed = 0;

        for document in documents {
            let tantivy_doc = document.to_tantivy_doc(&self.schema);

            // Delete existing document with same ID
            if let Ok(id_field) = self.schema.get_field("id") {
                let term = tantivy::Term::from_field_text(id_field, &document.document_id());
                writer.delete_term(term);
            }

            // Add the document
            writer.add_document(tantivy_doc).map_err(|e| {
                SearchError::IndexingFailed(format!("Failed to add document {}: {}", indexed, e))
            })?;

            indexed += 1;
        }

        // Commit all documents
        writer
            .commit()
            .map_err(|e| SearchError::IndexingFailed(format!("Failed to commit batch: {}", e)))?;

        Ok(indexed)
    }

    /// Delete a document by ID
    pub async fn delete_document(&self, document_id: &str) -> SearchResult<()> {
        let mut writer = self.writer.write().await;

        if let Ok(id_field) = self.schema.get_field("id") {
            let term = tantivy::Term::from_field_text(id_field, document_id);
            writer.delete_term(term);

            // Commit if real-time indexing is enabled
            if self.config.realtime_indexing {
                writer.commit().map_err(|e| {
                    SearchError::DeletionFailed(format!("Failed to commit deletion: {}", e))
                })?;
            }
        }

        Ok(())
    }

    /// Delete multiple documents by IDs
    pub async fn delete_documents(&self, document_ids: &[String]) -> SearchResult<usize> {
        let mut writer = self.writer.write().await;
        let mut deleted = 0;

        if let Ok(id_field) = self.schema.get_field("id") {
            for document_id in document_ids {
                let term = tantivy::Term::from_field_text(id_field, document_id);
                writer.delete_term(term);
                deleted += 1;
            }

            // Commit all deletions
            writer.commit().map_err(|e| {
                SearchError::DeletionFailed(format!("Failed to commit deletions: {}", e))
            })?;
        }

        Ok(deleted)
    }

    /// Commit pending changes
    pub async fn commit(&self) -> SearchResult<()> {
        let mut writer = self.writer.write().await;
        writer
            .commit()
            .map_err(|e| SearchError::IndexingFailed(format!("Failed to commit: {}", e)))?;
        Ok(())
    }

    /// Clear the entire index
    pub async fn clear_index(&self) -> SearchResult<()> {
        let mut writer = self.writer.write().await;
        writer.delete_all_documents().map_err(|e| {
            SearchError::IndexingFailed(format!("Failed to clear index: {}", e))
        })?;
        writer
            .commit()
            .map_err(|e| SearchError::IndexingFailed(format!("Failed to commit clear: {}", e)))?;
        Ok(())
    }

    /// Get index statistics
    pub async fn get_stats(&self) -> SearchResult<IndexStats> {
        let searcher = self.reader.searcher();

        // Count total documents
        let total_documents = searcher
            .search(&tantivy::query::AllQuery, &Count)
            .map_err(|e| SearchError::SearchFailed(format!("Failed to count documents: {}", e)))?
            as u64;

        // Get segment info
        let segment_metas = searcher.segment_readers();
        let num_segments = segment_metas.len();

        // Calculate approximate index size
        let index_size_bytes = std::fs::read_dir(&self.config.index_path)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| e.metadata().ok())
                    .map(|m| m.len())
                    .sum()
            })
            .unwrap_or(0);

        Ok(IndexStats {
            total_documents,
            index_size_bytes,
            num_segments,
            last_commit: Some(chrono::Utc::now()), // Simplified - would track actual commits
        })
    }

    /// Optimize the index (merge segments)
    pub async fn optimize(&self) -> SearchResult<()> {
        // Commit to trigger merging, then wait for merge policy to complete
        // Note: wait_merging_threads() takes ownership, so we can't call it through RwLock
        // Instead, we just commit which will trigger the merge policy
        let mut writer = self.writer.write().await;
        writer.commit().map_err(|e| {
            SearchError::IndexingFailed(format!("Failed to commit for optimization: {}", e))
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_index_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchConfig {
            index_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = IndexManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_index_stats() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchConfig {
            index_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = IndexManager::new(config).await.unwrap();
        let stats = manager.get_stats().await.unwrap();

        assert_eq!(stats.total_documents, 0);
    }
}
