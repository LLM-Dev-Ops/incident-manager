/// Correlation module for detecting and managing incident correlations
///
/// This module provides:
/// - Correlation detection strategies (temporal, pattern, source, fingerprint, topology)
/// - Correlation grouping and management
/// - Background correlation monitoring
/// - Manual correlation support

pub mod engine;
pub mod models;
pub mod strategy;

pub use engine::{CorrelationEngine, CorrelationStats};
pub use models::{
    Correlation, CorrelationConfig, CorrelationGroup, CorrelationResult, CorrelationType,
    GroupStatus,
};
pub use strategy::{
    CombinedStrategy, CorrelationStrategy, FingerprintStrategy, PatternStrategy, SourceStrategy,
    TemporalStrategy, TopologyStrategy,
};
