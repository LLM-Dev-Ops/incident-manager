//! Main search service implementation

use crate::models::Incident;
use crate::search::config::SearchConfig;
use crate::search::document::IncidentDocument;
use crate::search::error::{SearchError, SearchResult};
use crate::search::index::{IndexManager, IndexStats};
use crate::search::query::{Aggregation, FacetCount, QueryBuilder, SearchQuery};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::collector::{Count, FacetCollector, TopDocs};
use tantivy::query::Query;
use tantivy::schema::Value;
use tantivy::TantivyDocument;

/// A single search result hit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    /// Incident ID
    pub id: String,

    /// Incident title
    pub title: String,

    /// Incident description
    pub description: String,

    /// Severity
    pub severity: String,

    /// Incident type
    pub incident_type: String,

    /// Current state
    pub state: String,

    /// Source
    pub source: String,

    /// Assignee
    pub assignee: Option<String>,

    /// Tags
    pub tags: Vec<String>,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Search score/relevance
    pub score: f32,

    /// Highlighted snippets (if highlighting enabled)
    pub highlights: HashMap<String, Vec<String>>,
}

/// Search response with results and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Search results
    pub hits: Vec<SearchHit>,

    /// Total number of hits (before pagination)
    pub total_hits: usize,

    /// Search query that was executed
    pub query: String,

    /// Facet counts (if aggregations were requested)
    pub facets: HashMap<String, Vec<FacetCount>>,

    /// Search execution time in milliseconds
    pub search_time_ms: u64,

    /// Offset used for pagination
    pub offset: usize,

    /// Limit used for pagination
    pub limit: usize,
}

/// Search suggestion for autocomplete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    /// Suggested query text
    pub text: String,

    /// Number of results for this suggestion
    pub count: usize,

    /// Relevance score
    pub score: f32,
}

/// Main search service
pub struct SearchService {
    /// Index manager
    index_manager: Arc<IndexManager>,

    /// Configuration
    config: SearchConfig,
}

impl SearchService {
    /// Create a new search service
    pub async fn new(config: SearchConfig) -> SearchResult<Self> {
        let index_manager = Arc::new(IndexManager::new(config.clone()).await?);

        Ok(Self {
            index_manager,
            config,
        })
    }

    /// Search for incidents
    pub async fn search(&self, query: &SearchQuery) -> SearchResult<SearchResponse> {
        let start_time = std::time::Instant::now();

        // Build the Tantivy query
        let query_builder = QueryBuilder::new(self.index_manager.schema().clone(), self.index_manager.index().clone());
        let tantivy_query = query_builder
            .build(query)
            .map_err(|e| SearchError::QueryParsingFailed(e.to_string()))?;

        // Get searcher
        let searcher = self.index_manager.reader().searcher();

        // Execute search with TopDocs collector
        let limit = query.limit.min(self.config.max_results);
        let collector = TopDocs::with_limit(limit).and_offset(query.offset);

        let top_docs = searcher
            .search(&*tantivy_query, &collector)
            .map_err(|e| SearchError::SearchFailed(format!("Search execution failed: {}", e)))?;

        // Count total hits
        let total_hits = searcher
            .search(&*tantivy_query, &Count)
            .map_err(|e| SearchError::SearchFailed(format!("Count failed: {}", e)))?;

        // Convert results to SearchHits
        let schema = self.index_manager.schema();
        let mut hits = Vec::new();

        for (score, doc_address) in top_docs {
            let retrieved_doc = searcher
                .doc(doc_address)
                .map_err(|e| SearchError::SearchFailed(format!("Failed to retrieve doc: {}", e)))?;

            let hit = self.doc_to_search_hit(&retrieved_doc, score, schema)?;
            hits.push(hit);
        }

        // Compute facets if requested
        let mut facets = HashMap::new();
        if !query.aggregations.is_empty() {
            facets = self.compute_facets(&*tantivy_query, &query.aggregations).await?;
        }

        let search_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(SearchResponse {
            hits,
            total_hits,
            query: query.query.clone(),
            facets,
            search_time_ms,
            offset: query.offset,
            limit: query.limit,
        })
    }

    /// Convert Tantivy document to SearchHit
    fn doc_to_search_hit(
        &self,
        doc: &TantivyDocument,
        score: f32,
        schema: &tantivy::schema::Schema,
    ) -> SearchResult<SearchHit> {
        let id = self.get_field_value(doc, schema, "id").unwrap_or_default();
        let title = self.get_field_value(doc, schema, "title").unwrap_or_default();
        let description = self
            .get_field_value(doc, schema, "description")
            .unwrap_or_default();
        let assignee = self.get_field_value(doc, schema, "assignee");

        // Extract facet values
        let severity = self
            .get_facet_value(doc, schema, "severity")
            .unwrap_or_else(|| "Unknown".to_string());
        let incident_type = self
            .get_facet_value(doc, schema, "incident_type")
            .unwrap_or_else(|| "Unknown".to_string());
        let state = self
            .get_facet_value(doc, schema, "state")
            .unwrap_or_else(|| "Unknown".to_string());
        let source = self
            .get_facet_value(doc, schema, "source")
            .unwrap_or_else(|| "Unknown".to_string());

        // Get tags (multi-valued field)
        let tags = self.get_multi_field_values(doc, schema, "tags");

        // Get created_at timestamp
        let created_at = self
            .get_date_field(doc, schema, "created_at")
            .unwrap_or_else(Utc::now);

        Ok(SearchHit {
            id,
            title,
            description,
            severity,
            incident_type,
            state,
            source,
            assignee,
            tags,
            created_at,
            score,
            highlights: HashMap::new(), // TODO: Implement highlighting
        })
    }

    /// Get text field value from document
    fn get_field_value(
        &self,
        doc: &TantivyDocument,
        schema: &tantivy::schema::Schema,
        field_name: &str,
    ) -> Option<String> {
        schema.get_field(field_name).ok().and_then(|field| {
            doc.get_first(field)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
    }

    /// Get multi-valued text field
    fn get_multi_field_values(
        &self,
        doc: &TantivyDocument,
        schema: &tantivy::schema::Schema,
        field_name: &str,
    ) -> Vec<String> {
        schema
            .get_field(field_name)
            .ok()
            .map(|field| {
                doc.get_all(field)
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get facet value from document
    fn get_facet_value(
        &self,
        doc: &TantivyDocument,
        schema: &tantivy::schema::Schema,
        field_name: &str,
    ) -> Option<String> {
        schema.get_field(field_name).ok().and_then(|field| {
            doc.get_first(field).and_then(|v| {
                v.as_facet().map(|facet| {
                    // Extract the last component of the facet path
                    // e.g., "/severity/P0" -> "P0"
                    facet
                        .to_string()
                        .split('/')
                        .last()
                        .unwrap_or("")
                        .to_string()
                })
            })
        })
    }

    /// Get date field value
    fn get_date_field(
        &self,
        doc: &TantivyDocument,
        schema: &tantivy::schema::Schema,
        field_name: &str,
    ) -> Option<DateTime<Utc>> {
        schema.get_field(field_name).ok().and_then(|field| {
            doc.get_first(field).and_then(|v| {
                v.as_datetime().map(|dt| {
                    DateTime::from_timestamp(dt.into_timestamp_secs(), 0).unwrap_or_else(Utc::now)
                })
            })
        })
    }

    /// Compute facet aggregations
    async fn compute_facets(
        &self,
        query: &dyn Query,
        aggregations: &[Aggregation],
    ) -> SearchResult<HashMap<String, Vec<FacetCount>>> {
        let schema = self.index_manager.schema();
        let searcher = self.index_manager.reader().searcher();
        let mut results = HashMap::new();

        for aggregation in aggregations {
            let (facet_name, field_name) = match aggregation {
                Aggregation::BySeverity => ("severity", "severity"),
                Aggregation::ByIncidentType => ("incident_type", "incident_type"),
                Aggregation::ByState => ("state", "state"),
                Aggregation::BySource => ("source", "source"),
                Aggregation::ByAssignee => continue, // Skip for now (not a facet field)
                Aggregation::Custom(name) => (name.as_str(), name.as_str()),
            };

            if let Ok(_field) = schema.get_field(field_name) {
                let mut facet_collector = FacetCollector::for_field(field_name);
                facet_collector.add_facet(tantivy::schema::Facet::from("/"));

                let facet_counts = searcher
                    .search(query, &facet_collector)
                    .map_err(|e| {
                        SearchError::SearchFailed(format!("Facet aggregation failed: {}", e))
                    })?;

                let mut counts = Vec::new();
                for (facet, count) in facet_counts.get("/") {
                    // Extract the category name from the facet path
                    let facet_str = facet.to_string();
                    let name = facet_str.split('/').last().unwrap_or("");
                    if !name.is_empty() {
                        counts.push(FacetCount {
                            name: name.to_string(),
                            count,
                        });
                    }
                }

                // Sort by count descending
                counts.sort_by(|a, b| b.count.cmp(&a.count));
                results.insert(facet_name.to_string(), counts);
            }
        }

        Ok(results)
    }

    /// Index a single incident
    pub async fn index_incident(&self, incident: &Incident) -> SearchResult<()> {
        let document = IncidentDocument::from(incident);
        self.index_manager.index_document(&document).await
    }

    /// Index multiple incidents
    pub async fn index_incidents(&self, incidents: &[Incident]) -> SearchResult<usize> {
        let documents: Vec<_> = incidents.iter().map(IncidentDocument::from).collect();
        self.index_manager.index_documents(&documents).await
    }

    /// Delete an incident from the index
    pub async fn delete_incident(&self, incident_id: &str) -> SearchResult<()> {
        self.index_manager.delete_document(incident_id).await
    }

    /// Update an incident in the index
    pub async fn update_incident(&self, incident: &Incident) -> SearchResult<()> {
        // Delete and re-index (Tantivy handles this atomically)
        self.index_incident(incident).await
    }

    /// Get search suggestions based on partial query
    pub async fn suggest(&self, partial_query: &str, limit: usize) -> SearchResult<Vec<SearchSuggestion>> {
        if !self.config.enable_suggestions {
            return Ok(Vec::new());
        }

        // Simple implementation: search and return unique terms
        let query = SearchQuery::new(partial_query)
            .with_limit(limit)
            .with_highlight(false);

        let results = self.search(&query).await?;

        let mut suggestions = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for hit in results.hits {
            // Extract words from title
            for word in hit.title.split_whitespace() {
                let normalized = word.to_lowercase();
                if normalized.starts_with(&partial_query.to_lowercase())
                    && !seen.contains(&normalized)
                    && seen.len() < limit
                {
                    seen.insert(normalized.clone());
                    suggestions.push(SearchSuggestion {
                        text: word.to_string(),
                        count: 1, // Would need proper counting
                        score: hit.score,
                    });
                }
            }
        }

        suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        suggestions.truncate(limit);

        Ok(suggestions)
    }

    /// Aggregate incidents by a specific field
    pub async fn aggregate(
        &self,
        query: &SearchQuery,
        aggregation: Aggregation,
    ) -> SearchResult<Vec<FacetCount>> {
        let query_builder = QueryBuilder::new(self.index_manager.schema().clone(), self.index_manager.index().clone());
        let tantivy_query = query_builder
            .build(query)
            .map_err(|e| SearchError::QueryParsingFailed(e.to_string()))?;

        let facets = self.compute_facets(&*tantivy_query, &[aggregation]).await?;

        Ok(facets.values().next().cloned().unwrap_or_default())
    }

    /// Get index statistics
    pub async fn get_stats(&self) -> SearchResult<IndexStats> {
        self.index_manager.get_stats().await
    }

    /// Commit pending changes
    pub async fn commit(&self) -> SearchResult<()> {
        self.index_manager.commit().await
    }

    /// Clear the entire index
    pub async fn clear_index(&self) -> SearchResult<()> {
        self.index_manager.clear_index().await
    }

    /// Optimize the index (merge segments)
    pub async fn optimize(&self) -> SearchResult<()> {
        self.index_manager.optimize().await
    }

    /// Rebuild the entire index from incidents
    pub async fn rebuild_index(&self, incidents: &[Incident]) -> SearchResult<()> {
        // Clear existing index
        self.clear_index().await?;

        // Index all incidents
        self.index_incidents(incidents).await?;

        // Optimize
        self.optimize().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentState, IncidentType, Severity};
    use std::collections::HashMap;
    use tempfile::TempDir;

    async fn create_test_service() -> SearchService {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchConfig {
            index_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        SearchService::new(config).await.unwrap()
    }

    fn create_test_incident(id: &str, title: &str, severity: Severity) -> Incident {
        Incident {
            id: id.to_string(),
            title: title.to_string(),
            description: "Test description".to_string(),
            severity,
            incident_type: IncidentType::Infrastructure,
            state: IncidentState::Open,
            source: "test".to_string(),
            assignee: None,
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            resolved_at: None,
            correlation_id: None,
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_service_creation() {
        let service = create_test_service().await;
        let stats = service.get_stats().await.unwrap();
        assert_eq!(stats.total_documents, 0);
    }

    #[tokio::test]
    async fn test_index_and_search() {
        let service = create_test_service().await;

        // Index an incident
        let incident = create_test_incident("test-1", "Database connection error", Severity::P0);
        service.index_incident(&incident).await.unwrap();

        // Commit to make it searchable
        service.commit().await.unwrap();

        // Search for it
        let query = SearchQuery::new("database");
        let results = service.search(&query).await.unwrap();

        assert_eq!(results.total_hits, 1);
        assert_eq!(results.hits[0].id, "test-1");
    }

    #[tokio::test]
    async fn test_faceted_search() {
        let service = create_test_service().await;

        // Index multiple incidents
        let incidents = vec![
            create_test_incident("test-1", "Error 1", Severity::P0),
            create_test_incident("test-2", "Error 2", Severity::P1),
            create_test_incident("test-3", "Error 3", Severity::P0),
        ];

        service.index_incidents(&incidents).await.unwrap();
        service.commit().await.unwrap();

        // Search with severity aggregation
        let query = SearchQuery::new("error").with_aggregation(Aggregation::BySeverity);

        let results = service.search(&query).await.unwrap();

        assert_eq!(results.total_hits, 3);
        assert!(results.facets.contains_key("severity"));

        let severity_facets = &results.facets["severity"];
        assert!(severity_facets.iter().any(|f| f.name == "P0" && f.count == 2));
    }
}
