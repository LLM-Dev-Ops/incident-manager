/// Context enrichment module for incidents
///
/// This module provides context enrichment capabilities:
/// - Historical enrichment from similar past incidents
/// - Service context from service catalogs/CMDBs
/// - Team context with on-call information
/// - External API enrichments
/// - Parallel enrichment pipeline
/// - Caching for performance

pub mod enrichers;
pub mod models;
pub mod pipeline;
pub mod service;

pub use enrichers::{
    Enricher, ExternalApiEnricher, HistoricalEnricher, ServiceEnricher, TeamEnricher,
};
pub use models::{
    EnrichedContext, EnrichmentConfig, HistoricalContext, ServiceContext, TeamContext,
};
pub use pipeline::{CacheStats, EnrichmentPipeline};
pub use service::{EnrichmentService, EnrichmentStats};
