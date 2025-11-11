# Correlation Engine User Guide

## Overview

The Correlation Engine is an intelligent system that automatically detects relationships between incidents, groups related incidents together, and helps operators understand the scope and impact of problems across your infrastructure.

## Key Features

- **Automatic Correlation Detection**: Detects relationships between incidents using multiple strategies
- **Real-time Analysis**: Analyzes new incidents as they arrive
- **Intelligent Grouping**: Creates and maintains groups of related incidents
- **Multiple Correlation Types**: Temporal, pattern, source, fingerprint, topology, and combined correlations
- **Manual Correlation Support**: Allows operators to manually link incidents
- **Background Monitoring**: Continuous monitoring and maintenance of correlation state
- **Configurable Thresholds**: Fine-tune correlation sensitivity

## Correlation Strategies

### 1. Temporal Correlation

Detects incidents that occur close together in time.

**Use Case**: Cascading failures, domino effects

**Algorithm**: Exponential decay scoring based on time difference
```
score = e^(-decay_rate × time_diff)
```

**Example**:
- 09:00:00 - Database connection timeout
- 09:00:15 - API latency spike
- 09:00:30 - Frontend errors

These incidents are temporally correlated and likely part of the same outage.

### 2. Pattern Correlation

Detects incidents with similar content (titles, descriptions).

**Use Case**: Similar errors across different systems

**Algorithm**: Jaccard similarity + Levenshtein distance
```
score = (title_sim × 0.6) + (desc_sim × 0.3) + severity_match + type_match
```

**Example**:
- "Database connection pool exhausted"
- "Database connection pool full"

These incidents describe the same problem and should be correlated.

### 3. Source Correlation

Detects incidents from the same monitoring system or source.

**Use Case**: Problems with a specific monitoring tool or infrastructure component

**Algorithm**: Exact source match + timestamp proximity

**Example**:
- Source: "datadog" - Memory alert
- Source: "datadog" - CPU alert
- Source: "datadog" - Disk alert

Multiple alerts from the same source may indicate a broader infrastructure problem.

### 4. Fingerprint Correlation

Detects duplicate or highly similar incidents using fingerprints.

**Use Case**: Duplicate detection, recurring incidents

**Algorithm**: Exact fingerprint matching

**Example**:
- Fingerprint: "db-conn-timeout-prod-us-east"
- Multiple occurrences of the same root cause

### 5. Topology Correlation

Detects incidents affecting related infrastructure components.

**Use Case**: Infrastructure dependency chains

**Algorithm**: Graph-based topology analysis

**Example**:
- Database server failure
- Application servers depending on that database
- Load balancers routing to those application servers

### 6. Combined Correlation

Detects incidents matching multiple signals simultaneously.

**Use Case**: High-confidence correlations

**Algorithm**: Weighted combination of multiple strategies
```
score = (temporal × 0.3) + (pattern × 0.3) + (source × 0.2) + (fingerprint × 0.2)
boost = 1.2 if multiple_signals
```

## Configuration

### Basic Configuration

```toml
[processing]
# Enable correlation engine
correlation_enabled = true

# Time window for temporal correlation (seconds)
temporal_window_secs = 300  # 5 minutes

# Minimum score threshold to consider incidents correlated
min_correlation_score = 0.5  # 0.0 - 1.0

# Maximum incidents per group
max_group_size = 100

# Enable/disable specific strategies
enable_temporal = true
enable_pattern = true
enable_source = true
enable_fingerprint = true
enable_topology = false  # Requires topology data

# Pattern matching threshold
pattern_similarity_threshold = 0.7

# Auto-merge groups with high correlation
auto_merge_groups = true
merge_threshold = 0.8
```

### Configuration Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `correlation_enabled` | bool | true | Enable/disable correlation engine |
| `temporal_window_secs` | u64 | 300 | Time window for temporal correlation |
| `min_correlation_score` | f64 | 0.5 | Minimum score threshold (0.0-1.0) |
| `max_group_size` | usize | 100 | Maximum incidents per group |
| `enable_temporal` | bool | true | Enable temporal correlation |
| `enable_pattern` | bool | true | Enable pattern correlation |
| `enable_source` | bool | true | Enable source correlation |
| `enable_fingerprint` | bool | true | Enable fingerprint correlation |
| `enable_topology` | bool | false | Enable topology correlation |
| `pattern_similarity_threshold` | f64 | 0.7 | Pattern matching threshold |
| `auto_merge_groups` | bool | true | Auto-merge related groups |
| `merge_threshold` | f64 | 0.8 | Threshold for auto-merging |

## Usage

### Starting the Engine

The correlation engine starts automatically with the application when `correlation_enabled = true`.

```bash
# Start with default configuration
./llm-incident-manager

# Start with custom configuration
./llm-incident-manager --config config.toml
```

### Viewing Correlations

#### Get Correlation Group for an Incident

```bash
GET /v1/incidents/{incident_id}/correlation
```

Response:
```json
{
  "group_id": "550e8400-e29b-41d4-a716-446655440000",
  "title": "Correlated: Database connection failure",
  "primary_incident_id": "123e4567-e89b-12d3-a456-426614174000",
  "related_incident_ids": [
    "234e5678-e89b-12d3-a456-426614174001",
    "345e6789-e89b-12d3-a456-426614174002"
  ],
  "status": "active",
  "aggregate_score": 0.85,
  "size": 3,
  "created_at": "2025-01-15T09:00:00Z",
  "updated_at": "2025-01-15T09:05:00Z"
}
```

#### List All Active Correlation Groups

```bash
GET /v1/correlations/groups?status=active
```

Response:
```json
{
  "groups": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "Correlated: Database outage",
      "size": 5,
      "aggregate_score": 0.92,
      "status": "active"
    },
    {
      "id": "660e8400-e29b-41d4-a716-446655440001",
      "title": "Correlated: Network latency spike",
      "size": 3,
      "aggregate_score": 0.78,
      "status": "active"
    }
  ]
}
```

### Manual Correlation

Sometimes operators identify relationships that the automated system missed.

```bash
POST /v1/correlations/manual
```

Request:
```json
{
  "incident_ids": [
    "123e4567-e89b-12d3-a456-426614174000",
    "234e5678-e89b-12d3-a456-426614174001"
  ],
  "reason": "Both incidents caused by the same underlying network issue"
}
```

Response:
```json
{
  "correlation_id": "770e8400-e29b-41d4-a716-446655440002",
  "score": 1.0,
  "type": "manual",
  "created_at": "2025-01-15T10:00:00Z"
}
```

### Resolving Correlation Groups

When an incident group is resolved, mark the entire group as resolved:

```bash
POST /v1/correlations/groups/{group_id}/resolve
```

## Interpreting Correlation Results

### Correlation Scores

Correlation scores range from 0.0 (no correlation) to 1.0 (perfect correlation).

| Score Range | Interpretation | Action |
|-------------|----------------|--------|
| 0.9 - 1.0 | Very strong correlation | Almost certainly related |
| 0.7 - 0.9 | Strong correlation | Likely related |
| 0.5 - 0.7 | Moderate correlation | Possibly related |
| 0.3 - 0.5 | Weak correlation | Review manually |
| 0.0 - 0.3 | Very weak/no correlation | Likely unrelated |

### Group Status

| Status | Description | Meaning |
|--------|-------------|---------|
| Active | Currently accumulating incidents | Group is growing |
| Stable | No new incidents recently | Group has stabilized |
| Resolved | All incidents resolved | Problem fixed |
| Archived | Old resolved group | Historical data |

## Best Practices

### 1. Tuning Correlation Sensitivity

**Too Many False Positives** (unrelated incidents grouped):
- Increase `min_correlation_score` (try 0.7)
- Decrease `temporal_window_secs` (try 180)
- Increase `pattern_similarity_threshold` (try 0.8)

**Too Many False Negatives** (related incidents not grouped):
- Decrease `min_correlation_score` (try 0.4)
- Increase `temporal_window_secs` (try 600)
- Decrease `pattern_similarity_threshold` (try 0.6)

### 2. Using Manual Correlations

Manual correlations are valuable for:
- Training: Help improve automated correlation over time
- Edge cases: Relationships the system can't automatically detect
- Cross-system dependencies: Links that require domain knowledge

### 3. Monitoring Correlation Health

Key metrics to watch:
- **Group sizes**: Very large groups (>50) may indicate over-correlation
- **Group churn**: Frequent group creation/merging may indicate tuning issues
- **Manual correlation rate**: High rates suggest automation gaps

### 4. Correlation in Incident Response

During an outage:
1. **Identify the primary incident**: The root cause
2. **Review the correlation group**: See all affected systems
3. **Assess scope and impact**: Use group size and aggregate score
4. **Resolve as a unit**: Fix the root cause, mark group as resolved

## Troubleshooting

### Problem: No Correlations Detected

**Causes**:
- Correlation engine disabled
- Thresholds too high
- Incidents too dissimilar

**Solutions**:
```toml
[processing]
correlation_enabled = true
min_correlation_score = 0.5
temporal_window_secs = 300
```

### Problem: Too Many Spurious Correlations

**Causes**:
- Thresholds too low
- Temporal window too large

**Solutions**:
```toml
[processing]
min_correlation_score = 0.7
temporal_window_secs = 180
pattern_similarity_threshold = 0.8
```

### Problem: Engine Not Starting

Check logs:
```bash
grep "correlation" /var/log/llm-incident-manager.log
```

Common issues:
- Configuration errors
- Storage backend unavailable
- Permission issues

### Problem: Memory Usage Growing

**Causes**:
- Too many active groups
- Groups not being resolved
- Large temporal window

**Solutions**:
1. Resolve old groups regularly
2. Enable auto-merging
3. Reduce temporal window
4. Set `max_group_size`

## Performance Characteristics

### Latency

| Operation | Typical Latency | Notes |
|-----------|-----------------|-------|
| Analyze incident | 10-50ms | Depends on candidate count |
| Get group | <1ms | In-memory lookup |
| Manual correlate | <5ms | Direct storage |
| Background maintenance | 60s intervals | Non-blocking |

### Throughput

- **Incident analysis**: 1000+ incidents/second
- **Concurrent analysis**: Fully thread-safe
- **Memory per group**: ~1KB
- **Memory per correlation**: ~500 bytes

### Scalability

- Tested with 100,000+ incidents
- Tested with 10,000+ active groups
- Linear memory growth with group count
- Constant-time group lookups

## API Reference

### Get Correlation Statistics

```bash
GET /v1/correlations/stats
```

Response:
```json
{
  "total_groups": 145,
  "active_groups": 23,
  "stable_groups": 87,
  "resolved_groups": 35,
  "total_correlations": 532,
  "total_mapped_incidents": 1247
}
```

### Get Correlations for Incident

```bash
GET /v1/incidents/{incident_id}/correlations
```

Response:
```json
{
  "correlations": [
    {
      "id": "770e8400-e29b-41d4-a716-446655440002",
      "type": "temporal",
      "score": 0.89,
      "reason": "Incidents occurred within 45 seconds",
      "detected_at": "2025-01-15T09:00:45Z"
    },
    {
      "id": "880e8400-e29b-41d4-a716-446655440003",
      "type": "pattern",
      "score": 0.76,
      "reason": "Similar incident descriptions",
      "detected_at": "2025-01-15T09:00:45Z"
    }
  ]
}
```

## Advanced Topics

### Custom Correlation Strategies

Extend the correlation engine with custom strategies:

```rust
use llm_incident_manager::correlation::{CorrelationStrategy, Correlation};

pub struct CustomStrategy {
    // Your custom parameters
}

#[async_trait]
impl CorrelationStrategy for CustomStrategy {
    async fn correlate(
        &self,
        incident1: &Incident,
        incident2: &Incident,
        config: &CorrelationConfig,
    ) -> Result<Option<Correlation>> {
        // Your custom correlation logic
    }

    fn name(&self) -> &str {
        "custom"
    }

    fn correlation_type(&self) -> CorrelationType {
        CorrelationType::Combined
    }
}
```

### Topology Integration

To enable topology-based correlation, provide topology data:

```rust
// Configure topology relationships
let topology = TopologyGraph::new();
topology.add_edge("service-a", "database-1");
topology.add_edge("service-b", "database-1");

// Create engine with topology
let engine = CorrelationEngine::with_topology(config, store, topology);
```

## FAQ

**Q: How does correlation affect incident notifications?**

A: Correlations don't directly affect notifications. However, you can configure notification rules to suppress alerts for related incidents in a group.

**Q: Can I disable correlation for specific incident types?**

A: Currently, correlation applies to all incidents. You can filter by severity or source in the configuration.

**Q: What happens to correlations when incidents are deleted?**

A: Correlations involving deleted incidents are automatically removed during background maintenance.

**Q: How are correlation scores calculated?**

A: Each strategy calculates a score based on its specific algorithm. Combined correlations use weighted averages with bonuses for multi-signal matches.

**Q: Can correlation groups span multiple days?**

A: Yes, groups can span any time period. They remain active until explicitly resolved or stabilized.

**Q: How do I export correlation data?**

A: Use the correlation API endpoints to export data in JSON format, or query the storage backend directly.

## Support

For issues or questions:
- GitHub Issues: https://github.com/your-org/llm-incident-manager/issues
- Documentation: https://docs.example.com/correlation
- Slack: #llm-incident-manager

## See Also

- [Storage Guide](STORAGE_GUIDE.md)
- [Escalation Guide](ESCALATION_GUIDE.md)
- [API Reference](API_REFERENCE.md)
