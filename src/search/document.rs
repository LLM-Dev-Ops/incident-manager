//! Search document structures and indexing

use crate::models::Incident;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tantivy::schema::*;
use tantivy::TantivyDocument;

/// Trait for documents that can be indexed and searched
pub trait SearchDocument {
    /// Convert to Tantivy document
    fn to_tantivy_doc(&self, schema: &Schema) -> TantivyDocument;

    /// Get document ID
    fn document_id(&self) -> String;
}

/// Incident document for search indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentDocument {
    /// Incident ID
    pub id: String,

    /// Incident title
    pub title: String,

    /// Incident description
    pub description: String,

    /// Severity (P0, P1, P2, P3, P4)
    pub severity: String,

    /// Incident type
    pub incident_type: String,

    /// Current state
    pub state: String,

    /// Source system
    pub source: String,

    /// Assigned user/team
    pub assignee: Option<String>,

    /// Tags
    pub tags: Vec<String>,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,

    /// Resolved timestamp
    pub resolved_at: Option<DateTime<Utc>>,

    /// Correlation ID
    pub correlation_id: Option<String>,

    /// Metadata (indexed as JSON text)
    pub metadata: String,
}

impl From<&Incident> for IncidentDocument {
    fn from(incident: &Incident) -> Self {
        Self {
            id: incident.id.to_string(),
            title: incident.title.clone(),
            description: incident.description.clone(),
            severity: incident.severity.to_string(),
            incident_type: incident.incident_type.to_string(),
            state: incident.state.to_string(),
            source: incident.source.clone(),
            assignee: incident.assignees.first().cloned(),
            tags: incident.affected_resources.clone(),
            created_at: incident.created_at,
            updated_at: incident.updated_at,
            resolved_at: incident.resolution.as_ref().map(|r| r.resolved_at),
            correlation_id: incident.fingerprint.clone(),
            metadata: serde_json::to_string(&incident.labels).unwrap_or_default(),
        }
    }
}

impl From<Incident> for IncidentDocument {
    fn from(incident: Incident) -> Self {
        Self::from(&incident)
    }
}

impl SearchDocument for IncidentDocument {
    fn to_tantivy_doc(&self, schema: &Schema) -> TantivyDocument {
        let mut doc = TantivyDocument::new();

        // ID field
        if let Ok(field) = schema.get_field("id") {
            doc.add_text(field, &self.id);
        }

        // Title field (indexed and stored)
        if let Ok(field) = schema.get_field("title") {
            doc.add_text(field, &self.title);
        }

        // Description field (indexed and stored)
        if let Ok(field) = schema.get_field("description") {
            doc.add_text(field, &self.description);
        }

        // Severity (facet)
        if let Ok(field) = schema.get_field("severity") {
            doc.add_facet(field, Facet::from(&format!("/severity/{}", self.severity)));
        }

        // Incident type (facet)
        if let Ok(field) = schema.get_field("incident_type") {
            doc.add_facet(
                field,
                Facet::from(&format!("/incident_type/{}", self.incident_type)),
            );
        }

        // State (facet)
        if let Ok(field) = schema.get_field("state") {
            doc.add_facet(field, Facet::from(&format!("/state/{}", self.state)));
        }

        // Source (facet and text)
        if let Ok(field) = schema.get_field("source") {
            doc.add_facet(field, Facet::from(&format!("/source/{}", self.source)));
        }

        // Assignee
        if let Some(ref assignee) = self.assignee {
            if let Ok(field) = schema.get_field("assignee") {
                doc.add_text(field, assignee);
            }
        }

        // Tags (multi-valued)
        if let Ok(field) = schema.get_field("tags") {
            for tag in &self.tags {
                doc.add_text(field, tag);
            }
        }

        // Created timestamp
        if let Ok(field) = schema.get_field("created_at") {
            doc.add_date(field, tantivy::DateTime::from_timestamp_secs(self.created_at.timestamp()));
        }

        // Updated timestamp
        if let Ok(field) = schema.get_field("updated_at") {
            doc.add_date(field, tantivy::DateTime::from_timestamp_secs(self.updated_at.timestamp()));
        }

        // Resolved timestamp
        if let Some(resolved_at) = self.resolved_at {
            if let Ok(field) = schema.get_field("resolved_at") {
                doc.add_date(field, tantivy::DateTime::from_timestamp_secs(resolved_at.timestamp()));
            }
        }

        // Correlation ID
        if let Some(ref correlation_id) = self.correlation_id {
            if let Ok(field) = schema.get_field("correlation_id") {
                doc.add_text(field, correlation_id);
            }
        }

        // Metadata (as JSON text)
        if let Ok(field) = schema.get_field("metadata") {
            doc.add_text(field, &self.metadata);
        }

        doc
    }

    fn document_id(&self) -> String {
        self.id.clone()
    }
}

/// Build the search schema for incidents
pub fn build_incident_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // ID - stored, indexed as string
    schema_builder.add_text_field("id", STRING | STORED);

    // Title - full-text indexed with high boost, stored
    schema_builder.add_text_field("title", TEXT | STORED);

    // Description - full-text indexed, stored
    schema_builder.add_text_field("description", TEXT | STORED);

    // Severity - faceted field for filtering/aggregation
    schema_builder.add_facet_field("severity", INDEXED);

    // Incident type - faceted field
    schema_builder.add_facet_field("incident_type", INDEXED);

    // State - faceted field
    schema_builder.add_facet_field("state", INDEXED);

    // Source - faceted field
    schema_builder.add_facet_field("source", INDEXED);

    // Assignee - text field
    schema_builder.add_text_field("assignee", STRING | STORED);

    // Tags - multi-valued text field
    schema_builder.add_text_field("tags", TEXT | STORED);

    // Created timestamp - date field with fast access
    schema_builder.add_date_field("created_at", INDEXED | STORED | FAST);

    // Updated timestamp - date field
    schema_builder.add_date_field("updated_at", INDEXED | STORED | FAST);

    // Resolved timestamp - date field
    schema_builder.add_date_field("resolved_at", INDEXED | STORED | FAST);

    // Correlation ID
    schema_builder.add_text_field("correlation_id", STRING | STORED);

    // Metadata - JSON as text
    schema_builder.add_text_field("metadata", TEXT | STORED);

    schema_builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentState, IncidentType, Severity};

    #[test]
    fn test_incident_to_document() {
        let mut incident = Incident::new(
            "test".to_string(),
            "Test Incident".to_string(),
            "Test description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );
        incident.assignees.push("user@example.com".to_string());
        incident.affected_resources.push("tag1".to_string());
        incident.affected_resources.push("tag2".to_string());

        let doc = IncidentDocument::from(&incident);
        assert_eq!(doc.title, "Test Incident");
        assert_eq!(doc.severity, "P1");
    }

    #[test]
    fn test_schema_building() {
        let schema = build_incident_schema();
        assert!(schema.get_field("id").is_ok());
        assert!(schema.get_field("title").is_ok());
        assert!(schema.get_field("description").is_ok());
        assert!(schema.get_field("severity").is_ok());
    }
}
