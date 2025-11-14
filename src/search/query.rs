//! Search query building and parsing

use serde::{Deserialize, Serialize};

/// Sort order for search results
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// Field to sort by
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SearchSort {
    Relevance,
    CreatedAt(SortOrder),
    UpdatedAt(SortOrder),
    ResolvedAt(SortOrder),
}

impl Default for SearchSort {
    fn default() -> Self {
        Self::Relevance
    }
}

/// Search filter options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilter {
    /// Filter by severities
    pub severities: Option<Vec<String>>,

    /// Filter by incident types
    pub incident_types: Option<Vec<String>>,

    /// Filter by states
    pub states: Option<Vec<String>>,

    /// Filter by sources
    pub sources: Option<Vec<String>>,

    /// Filter by assignees
    pub assignees: Option<Vec<String>>,

    /// Filter by tags
    pub tags: Option<Vec<String>>,

    /// Filter by correlation ID
    pub correlation_id: Option<String>,

    /// Filter by date range (created_at)
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,

    /// Filter by date range (updated_at)
    pub updated_after: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_before: Option<chrono::DateTime<chrono::Utc>>,
}

/// Aggregation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Aggregation {
    /// Count by severity
    BySeverity,

    /// Count by incident type
    ByIncidentType,

    /// Count by state
    ByState,

    /// Count by source
    BySource,

    /// Count by assignee
    ByAssignee,

    /// Custom facet
    Custom(String),
}

/// Facet count result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetCount {
    pub name: String,
    pub count: u64,
}

/// Main search query structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// The search query text (supports full-text, phrase, boolean queries)
    pub query: String,

    /// Filters to apply
    pub filters: SearchFilter,

    /// Sorting criteria
    pub sort: SearchSort,

    /// Number of results to return
    pub limit: usize,

    /// Offset for pagination
    pub offset: usize,

    /// Aggregations to compute
    pub aggregations: Vec<Aggregation>,

    /// Enable highlighting
    pub highlight: bool,
}

impl SearchQuery {
    /// Create a new search query
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            filters: SearchFilter::default(),
            sort: SearchSort::default(),
            limit: 20,
            offset: 0,
            aggregations: Vec::new(),
            highlight: true,
        }
    }

    /// Set filters
    pub fn with_filters(mut self, filters: SearchFilter) -> Self {
        self.filters = filters;
        self
    }

    /// Filter by severity
    pub fn with_severity(mut self, severities: Vec<impl Into<String>>) -> Self {
        self.filters.severities = Some(severities.into_iter().map(|s| s.into()).collect());
        self
    }

    /// Filter by incident type
    pub fn with_incident_type(mut self, types: Vec<impl Into<String>>) -> Self {
        self.filters.incident_types = Some(types.into_iter().map(|t| t.into()).collect());
        self
    }

    /// Filter by state
    pub fn with_state(mut self, states: Vec<impl Into<String>>) -> Self {
        self.filters.states = Some(states.into_iter().map(|s| s.into()).collect());
        self
    }

    /// Filter by source
    pub fn with_source(mut self, sources: Vec<impl Into<String>>) -> Self {
        self.filters.sources = Some(sources.into_iter().map(|s| s.into()).collect());
        self
    }

    /// Filter by tags
    pub fn with_tags(mut self, tags: Vec<impl Into<String>>) -> Self {
        self.filters.tags = Some(tags.into_iter().map(|t| t.into()).collect());
        self
    }

    /// Filter by date range (created_at)
    pub fn with_created_range(
        mut self,
        after: Option<chrono::DateTime<chrono::Utc>>,
        before: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        self.filters.created_after = after;
        self.filters.created_before = before;
        self
    }

    /// Set sorting
    pub fn with_sort(mut self, sort: SearchSort) -> Self {
        self.sort = sort;
        self
    }

    /// Set limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set offset
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Add aggregation
    pub fn with_aggregation(mut self, aggregation: Aggregation) -> Self {
        self.aggregations.push(aggregation);
        self
    }

    /// Enable/disable highlighting
    pub fn with_highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }
}

/// Query builder for constructing complex Tantivy queries
pub struct QueryBuilder {
    schema: tantivy::schema::Schema,
    index: tantivy::Index,
}

impl QueryBuilder {
    /// Create a new query builder
    pub fn new(schema: tantivy::schema::Schema, index: tantivy::Index) -> Self {
        Self { schema, index }
    }

    /// Build a Tantivy query from a SearchQuery
    pub fn build(
        &self,
        search_query: &SearchQuery,
    ) -> Result<Box<dyn tantivy::query::Query>, tantivy::query::QueryParserError> {
        use tantivy::query::*;

        let mut subqueries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        // Main text query (search title, description, tags)
        if !search_query.query.is_empty() {
            let mut text_fields = Vec::new();

            if let Ok(title_field) = self.schema.get_field("title") {
                text_fields.push(title_field);
            }
            if let Ok(desc_field) = self.schema.get_field("description") {
                text_fields.push(desc_field);
            }
            if let Ok(tags_field) = self.schema.get_field("tags") {
                text_fields.push(tags_field);
            }

            let query_parser = QueryParser::for_index(&self.index, text_fields);
            let parsed_query = query_parser.parse_query(&search_query.query)?;
            subqueries.push((Occur::Must, parsed_query));
        }

        // Severity filter
        if let Some(ref severities) = search_query.filters.severities {
            if let Ok(severity_field) = self.schema.get_field("severity") {
                let mut severity_queries: Vec<Box<dyn Query>> = Vec::new();
                for severity in severities {
                    let facet = tantivy::schema::Facet::from(&format!("/severity/{}", severity));
                    severity_queries.push(Box::new(TermQuery::new(
                        tantivy::Term::from_facet(severity_field, &facet),
                        tantivy::schema::IndexRecordOption::Basic,
                    )));
                }
                if !severity_queries.is_empty() {
                    subqueries.push((Occur::Must, Box::new(DisjunctionMaxQuery::new(severity_queries))));
                }
            }
        }

        // Incident type filter
        if let Some(ref types) = search_query.filters.incident_types {
            if let Ok(type_field) = self.schema.get_field("incident_type") {
                let mut type_queries: Vec<Box<dyn Query>> = Vec::new();
                for incident_type in types {
                    let facet = tantivy::schema::Facet::from(&format!("/incident_type/{}", incident_type));
                    type_queries.push(Box::new(TermQuery::new(
                        tantivy::Term::from_facet(type_field, &facet),
                        tantivy::schema::IndexRecordOption::Basic,
                    )));
                }
                if !type_queries.is_empty() {
                    subqueries.push((Occur::Must, Box::new(DisjunctionMaxQuery::new(type_queries))));
                }
            }
        }

        // State filter
        if let Some(ref states) = search_query.filters.states {
            if let Ok(state_field) = self.schema.get_field("state") {
                let mut state_queries: Vec<Box<dyn Query>> = Vec::new();
                for state in states {
                    let facet = tantivy::schema::Facet::from(&format!("/state/{}", state));
                    state_queries.push(Box::new(TermQuery::new(
                        tantivy::Term::from_facet(state_field, &facet),
                        tantivy::schema::IndexRecordOption::Basic,
                    )));
                }
                if !state_queries.is_empty() {
                    subqueries.push((Occur::Must, Box::new(DisjunctionMaxQuery::new(state_queries))));
                }
            }
        }

        // Source filter
        if let Some(ref sources) = search_query.filters.sources {
            if let Ok(source_field) = self.schema.get_field("source") {
                let mut source_queries: Vec<Box<dyn Query>> = Vec::new();
                for source in sources {
                    let facet = tantivy::schema::Facet::from(&format!("/source/{}", source));
                    source_queries.push(Box::new(TermQuery::new(
                        tantivy::Term::from_facet(source_field, &facet),
                        tantivy::schema::IndexRecordOption::Basic,
                    )));
                }
                if !source_queries.is_empty() {
                    subqueries.push((Occur::Must, Box::new(DisjunctionMaxQuery::new(source_queries))));
                }
            }
        }

        // Date range filters
        if let Ok(_created_field) = self.schema.get_field("created_at") {
            if let Some(after) = search_query.filters.created_after {
                let range_query = RangeQuery::new_date_bounds(
                    "created_at".to_string(),
                    std::ops::Bound::Included(tantivy::DateTime::from_timestamp_secs(after.timestamp())),
                    std::ops::Bound::Unbounded,
                );
                subqueries.push((Occur::Must, Box::new(range_query)));
            }
            if let Some(before) = search_query.filters.created_before {
                let range_query = RangeQuery::new_date_bounds(
                    "created_at".to_string(),
                    std::ops::Bound::Unbounded,
                    std::ops::Bound::Included(tantivy::DateTime::from_timestamp_secs(before.timestamp())),
                );
                subqueries.push((Occur::Must, Box::new(range_query)));
            }
        }

        // Combine all queries
        if subqueries.is_empty() {
            // No query - return all documents
            Ok(Box::new(AllQuery))
        } else if subqueries.len() == 1 {
            Ok(subqueries.into_iter().next().unwrap().1)
        } else {
            Ok(Box::new(BooleanQuery::from(subqueries)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_builder() {
        let query = SearchQuery::new("database error")
            .with_severity(vec!["P0", "P1"])
            .with_limit(50)
            .with_offset(10);

        assert_eq!(query.query, "database error");
        assert_eq!(query.limit, 50);
        assert_eq!(query.offset, 10);
        assert_eq!(query.filters.severities.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_search_filter() {
        let filter = SearchFilter {
            severities: Some(vec!["P0".to_string()]),
            incident_types: Some(vec!["Infrastructure".to_string()]),
            ..Default::default()
        };

        assert!(filter.severities.is_some());
        assert!(filter.incident_types.is_some());
    }
}
