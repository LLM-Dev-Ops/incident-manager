# Context Enrichment Guide

## Overview

The Context Enrichment system automatically augments incidents with additional contextual information from multiple sources to provide comprehensive incident intelligence. This helps responders understand the full context of an incident, including historical patterns, service relationships, team information, and related operational data.

## Features

- **Historical Context**: Finds similar past incidents to provide resolution guidance
- **Service Context**: Enriches with service catalog/CMDB information
- **Team Context**: Adds team and on-call information
- **External API Integration**: Fetches data from external monitoring/observability systems
- **Parallel Execution**: Runs multiple enrichers concurrently for fast enrichment
- **Intelligent Caching**: Caches enrichment results with TTL for performance
- **Configurable Timeouts**: Prevents slow enrichers from blocking incident processing
- **Priority-based Ordering**: Executes enrichers in optimal order

## Quick Start

### Basic Setup

```rust
use llm_incident_manager::enrichment::{EnrichmentConfig, EnrichmentService};
use llm_incident_manager::state::InMemoryStore;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Create the incident store
    let store = Arc::new(InMemoryStore::new());

    // Configure enrichment
    let config = EnrichmentConfig {
        enabled: true,
        enable_historical: true,
        enable_service: true,
        enable_team: true,
        timeout_secs: 10,
        max_concurrent: 5,
        cache_ttl_secs: 300,
        ..Default::default()
    };

    // Create and start the enrichment service
    let enrichment_service = Arc::new(EnrichmentService::new(config, store));
    enrichment_service.start().await.unwrap();

    println!("Enrichment service started!");
}
```

### Enriching an Incident

```rust
// Create an incident
let incident = Incident::new(
    "monitoring".to_string(),
    "API Gateway Timeout".to_string(),
    "API gateway timing out on /api/payment requests".to_string(),
    Severity::P1,
    IncidentType::Application,
);

// Enrich the incident
let enriched_context = enrichment_service
    .enrich_incident(&incident)
    .await
    .unwrap();

// Access enriched data
if let Some(historical) = enriched_context.historical {
    println!("Found {} similar past incidents", historical.similar_incidents.len());
    for similar in &historical.similar_incidents {
        println!("  - {} (similarity: {:.2})", similar.title, similar.similarity_score);
    }
}

if let Some(service) = enriched_context.service {
    println!("Service: {}", service.service_name);
    println!("Owner: {}", service.owner_team);
}

if let Some(team) = enriched_context.team {
    println!("Team: {}", team.team_name);
    println!("On-call: {:?}", team.on_call_engineers);
}
```

## Configuration

### EnrichmentConfig

```rust
pub struct EnrichmentConfig {
    /// Enable/disable entire enrichment system
    pub enabled: bool,

    /// Enable historical incident analysis
    pub enable_historical: bool,

    /// Enable service catalog enrichment
    pub enable_service: bool,

    /// Enable team context enrichment
    pub enable_team: bool,

    /// Enable metrics enrichment
    pub enable_metrics: bool,

    /// Enable log aggregation enrichment
    pub enable_logs: bool,

    /// Timeout for each enricher (seconds)
    pub timeout_secs: u64,

    /// Maximum concurrent enrichers when async_enrichment is true
    pub max_concurrent: usize,

    /// Cache TTL (seconds)
    pub cache_ttl_secs: u64,

    /// Number of retry attempts for failed enrichments
    pub retry_attempts: usize,

    /// Historical lookback period (seconds)
    pub historical_lookback_secs: u64,

    /// Similarity threshold (0.0-1.0) for historical matching
    pub similarity_threshold: f64,

    /// External API endpoints (name -> URL)
    pub external_apis: HashMap<String, String>,

    /// Timeout for external API calls (seconds)
    pub api_timeout_secs: u64,

    /// Run enrichers in parallel vs sequential
    pub async_enrichment: bool,
}
```

### Default Configuration

```rust
let config = EnrichmentConfig::default();
// enabled: true
// enable_historical: true
// enable_service: true
// enable_team: true
// enable_metrics: false
// enable_logs: false
// timeout_secs: 10
// max_concurrent: 5
// cache_ttl_secs: 300 (5 minutes)
// retry_attempts: 2
// historical_lookback_secs: 2592000 (30 days)
// similarity_threshold: 0.5
// async_enrichment: true
```

## Enrichment Types

### 1. Historical Enrichment

Analyzes past incidents to find similar occurrences and provide resolution guidance.

```rust
// Historical context includes:
pub struct HistoricalContext {
    pub similar_incidents: Vec<SimilarIncident>,
    pub total_occurrences: usize,
    pub avg_resolution_time_mins: u64,
    pub common_resolution_steps: Vec<String>,
    pub lookback_period_secs: u64,
}
```

**Similarity Calculation**:
- Title similarity (40% weight) using Jaccard similarity
- Description similarity (30% weight) using Jaccard similarity
- Source match (15% weight)
- Type match (15% weight)

### 2. Service Enrichment

Enriches with service catalog/CMDB information.

```rust
pub struct ServiceContext {
    pub service_name: String,
    pub service_id: String,
    pub description: String,
    pub owner_team: String,
    pub tier: String,
    pub status: ServiceStatus,
    pub dependencies: Vec<ServiceDependency>,
    pub runbooks: Vec<String>,
    pub dashboards: Vec<String>,
    pub tags: HashMap<String, String>,
}
```

### 3. Team Enrichment

Adds team and on-call information.

```rust
pub struct TeamContext {
    pub team_name: String,
    pub team_id: String,
    pub escalation_policy: String,
    pub on_call_engineers: Vec<String>,
    pub slack_channel: Option<String>,
    pub email: Option<String>,
    pub managers: Vec<String>,
    pub timezone: String,
}
```

### 4. External API Enrichment

Fetches data from external systems (metrics, logs, traces).

```rust
// Configure external APIs
let mut config = EnrichmentConfig::default();
config.external_apis.insert(
    "prometheus".to_string(),
    "http://prometheus.example.com/api/v1/query".to_string()
);
config.external_apis.insert(
    "elasticsearch".to_string(),
    "http://elasticsearch.example.com/_search".to_string()
);
```

## Integration with IncidentProcessor

The enrichment service integrates seamlessly with the incident processor:

```rust
use llm_incident_manager::processing::IncidentProcessor;
use llm_incident_manager::enrichment::EnrichmentService;

// Create processor
let mut processor = IncidentProcessor::new(store.clone(), dedup_engine);

// Add enrichment service
processor.set_enrichment_service(enrichment_service);

// Process alerts - enrichment happens automatically
let alert = Alert::new(/* ... */);
let ack = processor.process_alert(alert).await.unwrap();
```

## Cache Management

### Cache Statistics

```rust
let stats = enrichment_service.get_cache_stats().await;
println!("Cache size: {}", stats.size);
println!("Cache capacity: {}", stats.capacity);
```

### Clearing Cache

```rust
// Clear all cached enrichments
enrichment_service.clear_cache().await;

// Cache also auto-expires based on TTL
// Background cleanup runs every 60 seconds
```

### Manual Cache Retrieval

```rust
// Get cached context for an incident
if let Some(context) = enrichment_service.get_context(&incident_id).await {
    println!("Found cached enrichment");
}
```

## Custom Enrichers

You can create and register custom enrichers:

```rust
use llm_incident_manager::enrichment::{Enricher, EnrichmentResult};
use async_trait::async_trait;

pub struct CustomEnricher {
    name: String,
}

#[async_trait]
impl Enricher for CustomEnricher {
    fn name(&self) -> &str {
        &self.name
    }

    async fn enrich(
        &self,
        incident: &Incident,
        context: &mut EnrichedContext,
        config: &EnrichmentConfig,
    ) -> EnrichmentResult {
        let start = std::time::Instant::now();

        // Your custom enrichment logic here
        context.metadata.insert("custom_field".to_string(), "custom_value".to_string());

        EnrichmentResult::success(
            self.name.clone(),
            start.elapsed().as_millis() as u64,
        )
    }

    fn is_enabled(&self, config: &EnrichmentConfig) -> bool {
        config.enabled
    }

    fn priority(&self) -> u32 {
        50 // Lower numbers run first
    }
}

// Register the custom enricher
let custom = Arc::new(CustomEnricher {
    name: "custom".to_string(),
});
enrichment_service.register_enricher(custom).await;
```

## Performance Tuning

### Parallel vs Sequential Execution

```rust
// Parallel execution (faster, more resource intensive)
config.async_enrichment = true;
config.max_concurrent = 5;

// Sequential execution (slower, less resource intensive)
config.async_enrichment = false;
```

### Timeout Configuration

```rust
// Per-enricher timeout
config.timeout_secs = 10;

// External API timeout
config.api_timeout_secs = 5;
```

### Cache Tuning

```rust
// Longer cache TTL = better performance, potentially stale data
config.cache_ttl_secs = 600; // 10 minutes

// Shorter cache TTL = fresher data, more enrichment calls
config.cache_ttl_secs = 60; // 1 minute
```

## Service Statistics

```rust
let stats = enrichment_service.get_stats().await;
println!("Enabled: {}", stats.enabled);
println!("Running: {}", stats.is_running);
println!("Total enrichers: {}", stats.total_enrichers);
println!("Enabled enrichers: {}", stats.enabled_enrichers);
println!("Cache size: {}", stats.cache_size);
println!("Async enrichment: {}", stats.async_enrichment);
```

## Error Handling

Enrichment failures don't block incident processing:

```rust
match enrichment_service.enrich_incident(&incident).await {
    Ok(context) => {
        println!("Enrichment succeeded");
        println!("Successful: {:?}", context.successful_enrichers);
        println!("Failed: {:?}", context.failed_enrichers);
    }
    Err(e) => {
        eprintln!("Enrichment failed: {}", e);
        // Incident processing continues
    }
}
```

## Best Practices

1. **Enable Only What You Need**: Disable unused enrichers to improve performance
2. **Set Appropriate Timeouts**: Balance between completeness and latency
3. **Use Parallel Execution**: Enable `async_enrichment` for production deployments
4. **Monitor Cache Hit Rate**: Adjust TTL based on your incident patterns
5. **Tune Similarity Threshold**: Adjust based on your historical data quality
6. **Custom Enrichers**: Keep them fast and focused on single responsibility
7. **External APIs**: Ensure they're reliable and have proper timeouts

## Troubleshooting

### Enrichment Taking Too Long

```rust
// Reduce timeout
config.timeout_secs = 5;

// Reduce concurrent enrichers
config.max_concurrent = 3;

// Disable slow enrichers
config.enable_logs = false;
```

### Too Many Cache Misses

```rust
// Increase cache TTL
config.cache_ttl_secs = 600;

// Check cache statistics
let stats = enrichment_service.get_cache_stats().await;
```

### Historical Enrichment Not Finding Matches

```rust
// Lower similarity threshold
config.similarity_threshold = 0.3;

// Increase lookback period
config.historical_lookback_secs = 3600 * 24 * 90; // 90 days
```

### External API Failures

```rust
// Check enrichment results
let context = enrichment_service.enrich_incident(&incident).await?;
println!("Failed enrichers: {:?}", context.failed_enrichers);

// Increase API timeout
config.api_timeout_secs = 10;
```

## Examples

### Example 1: Basic Enrichment

```rust
let store = Arc::new(InMemoryStore::new());
let config = EnrichmentConfig::default();
let service = EnrichmentService::new(config, store);

service.start().await?;

let incident = Incident::new(/* ... */);
let context = service.enrich_incident(&incident).await?;

println!("Enriched with {} enrichers in {}ms",
    context.total_enrichers(),
    context.enrichment_duration_ms);
```

### Example 2: Selective Enrichment

```rust
let mut config = EnrichmentConfig::default();
config.enable_historical = true;
config.enable_service = true;
config.enable_team = false;
config.enable_metrics = false;
config.enable_logs = false;

let service = EnrichmentService::new(config, store);
```

### Example 3: High-Performance Configuration

```rust
let mut config = EnrichmentConfig::default();
config.async_enrichment = true;
config.max_concurrent = 10;
config.timeout_secs = 5;
config.cache_ttl_secs = 600;

let service = EnrichmentService::new(config, store);
```

### Example 4: Development/Testing Configuration

```rust
let mut config = EnrichmentConfig::default();
config.async_enrichment = false; // Sequential for debugging
config.timeout_secs = 30; // Longer timeout
config.cache_ttl_secs = 60; // Shorter cache for testing

let service = EnrichmentService::new(config, store);
```

## API Reference

### EnrichmentService

- `new(config, store)` - Create new service
- `start()` - Start the service
- `stop()` - Stop the service
- `is_running()` - Check if running
- `enrich_incident(incident)` - Enrich an incident
- `get_context(incident_id)` - Get cached context
- `register_enricher(enricher)` - Register custom enricher
- `clear_cache()` - Clear all cached enrichments
- `get_cache_stats()` - Get cache statistics
- `get_stats()` - Get service statistics
- `update_config(config)` - Update configuration

### EnrichedContext

- `incident_id` - Incident UUID
- `historical` - Historical context
- `service` - Service context
- `team` - Team context
- `metrics` - Metrics context
- `logs` - Log context
- `metadata` - Custom metadata
- `enriched_at` - Enrichment timestamp
- `enrichment_duration_ms` - Enrichment duration
- `successful_enrichers` - List of successful enrichers
- `failed_enrichers` - List of failed enrichers
- `total_enrichers()` - Total enricher count

## See Also

- [Implementation Guide](ENRICHMENT_IMPLEMENTATION.md) - Technical details
- [API Documentation](API.md) - Full API reference
- [Architecture](ARCHITECTURE.md) - System architecture
