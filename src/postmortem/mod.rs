//! Post-Mortem Generation Module
//!
//! This module provides comprehensive post-incident analysis and documentation
//! capabilities for generating authoritative post-mortem records.
//!
//! # Agent Classification
//!
//! - **Primary**: DOCUMENTATION
//! - **Secondary**: INCIDENT ANALYSIS (POST-RESOLUTION ONLY)
//!
//! # Purpose
//!
//! Produce authoritative post-incident records that capture the complete incident
//! lifecycle, including timeline reconstruction, root cause analysis, impact assessment,
//! and actionable follow-up items.
//!
//! # Decision Type
//!
//! - `decision_type`: `"incident_postmortem_generated"`
//!
//! # Constraints
//!
//! This module MUST NOT:
//!
//! - **Modify incident state**: Post-mortems are read-only analysis of resolved incidents
//! - **Trigger remediation actions**: All remediation must be complete before post-mortem
//! - **Reassign severity**: Severity changes are outside post-mortem scope
//! - **Alter historical timeline**: Timeline is reconstructed from immutable event records
//! - **Generate during active incidents**: Only resolved/closed incidents are eligible
//!
//! # Integration with ruvector-service
//!
//! The module integrates with ruvector-service for:
//!
//! - **Decision persistence**: All generated post-mortems are persisted as decision events
//! - **Vector embeddings**: Post-mortem content is embedded for semantic search
//! - **Pattern learning**: Successful resolutions feed into the learning system
//! - **Audit trail**: Complete audit trail of post-mortem generation decisions
//!
//! # Components
//!
//! - [`models`]: Core data structures for post-mortem representation
//! - [`generator`]: Post-mortem generation logic and orchestration
//! - [`client`]: RuVector service client for persistence and retrieval
//!
//! # Example
//!
//! ```no_run
//! use llm_incident_manager::postmortem::{
//!     PostMortemGenerator, PostMortem, PostMortemStatus,
//!     RuvectorClient, RuvectorConfig,
//! };
//! use uuid::Uuid;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Configure ruvector client
//!     let config = RuvectorConfig::default();
//!     let client = RuvectorClient::new(config)?;
//!
//!     // Create generator with persistence
//!     let generator = PostMortemGenerator::new(client);
//!
//!     // Generate post-mortem for a resolved incident
//!     let incident_id = Uuid::new_v4();
//!     let postmortem = generator.generate(incident_id).await?;
//!
//!     assert_eq!(postmortem.status, PostMortemStatus::Draft);
//!     println!("Generated post-mortem: {}", postmortem.id);
//!
//!     Ok(())
//! }
//! ```

// Submodule declarations
pub mod client;
pub mod generator;
pub mod models;

// Re-export core types from models
pub use models::{
    ActionItem,
    ActionItemPriority,
    ActionItemStatus,
    DecisionEvent,
    ImpactAnalysis,
    PostMortem,
    PostMortemOutput,
    PostMortemStatus,
    ResolutionAnalysis,
    RootCauseAnalysis,
    TimelineEntry,
};

// Re-export generator
pub use generator::PostMortemGenerator;

// Re-export client types
pub use client::{PersistenceResponse, RuvectorClient, RuvectorConfig};
