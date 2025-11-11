# Persistent Storage Guide

## Overview

The LLM Incident Manager supports multiple persistent storage backends to meet different deployment requirements:

- **In-Memory**: Fast, for development and testing (no persistence)
- **Sled**: Embedded database, single-node deployment (persistent)
- **Redis**: Distributed cache/database, multi-node deployment (persistent)
- **Redis Cluster**: High-availability distributed storage (persistent)

## Table of Contents

1. [Storage Backends](#storage-backends)
2. [Configuration](#configuration)
3. [Deployment Scenarios](#deployment-scenarios)
4. [Performance Characteristics](#performance-characteristics)
5. [Data Model](#data-model)
6. [Migration](#migration)
7. [Backup and Recovery](#backup-and-recovery)
8. [Troubleshooting](#troubleshooting)

## Storage Backends

### In-Memory Store

**Use Case**: Development, testing, ephemeral workloads

**Characteristics**:
- No external dependencies
- Fastest performance
- Data lost on restart
- Single-node only
- No configuration required

**When to Use**:
- Local development
- Integration tests
- Short-lived preview environments
- POC/demo environments

**Configuration**:
```toml
[state]
backend = "sled"  # Any backend except in-memory is persistent
```

---

### Sled Store

**Use Case**: Single-node production deployments, edge computing

**Characteristics**:
- Embedded database (no external service required)
- ACID compliant
- Crash-safe with write-ahead logging
- Automatic background compaction
- ~100k ops/sec throughput
- Persistent across restarts
- Single-node only

**When to Use**:
- Single-server deployments
- Edge deployments
- Kubernetes StatefulSet with single replica
- Cost-sensitive deployments
- Air-gapped environments

**Configuration**:
```toml
[state]
backend = "sled"
path = "./data/sled"  # Required: path to database directory
pool_size = 100
```

**Pros**:
- ✅ No external dependencies
- ✅ Simple deployment
- ✅ ACID guarantees
- ✅ Low latency
- ✅ Automatic crash recovery

**Cons**:
- ❌ Single-node only
- ❌ No built-in replication
- ❌ Manual backup required

---

### Redis Store

**Use Case**: Distributed deployments, high-throughput systems

**Characteristics**:
- External Redis server required
- Supports Redis 5.0+
- ~1M ops/sec throughput (depending on Redis config)
- Built-in persistence (RDB + AOF)
- Optional replication
- Support for Redis Sentinel
- Efficient indexing with Redis Sets

**When to Use**:
- Multi-node deployments
- High-throughput requirements (>100k incidents/sec)
- Existing Redis infrastructure
- Need for data sharing across services
- Real-time analytics requirements

**Configuration**:
```toml
[state]
backend = "redis"
redis_url = "redis://localhost:6379/0"
pool_size = 100
```

**Connection String Examples**:
```bash
# Standard connection
redis://localhost:6379/0

# With authentication
redis://:password@localhost:6379/0

# With username and password (Redis 6+)
redis://username:password@localhost:6379/0

# TLS connection
rediss://localhost:6380/0

# Redis Sentinel
redis+sentinel://sentinel1:26379,sentinel2:26379,sentinel3:26379/mymaster/0
```

**Pros**:
- ✅ Distributed architecture
- ✅ High throughput
- ✅ Built-in replication
- ✅ Redis Sentinel for HA
- ✅ Optional persistence
- ✅ Rich ecosystem

**Cons**:
- ❌ External dependency
- ❌ Additional infrastructure
- ❌ Higher latency than Sled
- ❌ Memory-intensive

---

### Redis Cluster Store

**Use Case**: High-availability, large-scale deployments

**Characteristics**:
- Automatic sharding across nodes
- Built-in failover
- Horizontal scaling
- Minimum 6 nodes recommended (3 masters, 3 replicas)
- ~10M ops/sec throughput (cluster-wide)

**When to Use**:
- Large-scale deployments (>1M incidents)
- High-availability requirements (99.99%+)
- Need for horizontal scaling
- Geographic distribution

**Configuration**:
```toml
[state]
backend = "redis_cluster"
redis_cluster_nodes = [
    "redis://node1:6379",
    "redis://node2:6379",
    "redis://node3:6379",
    "redis://node4:6379",
    "redis://node5:6379",
    "redis://node6:6379",
]
pool_size = 100
```

**Pros**:
- ✅ Automatic sharding
- ✅ High availability
- ✅ Horizontal scaling
- ✅ Automatic failover
- ✅ Geographic distribution

**Cons**:
- ❌ Complex setup
- ❌ Higher operational cost
- ❌ Minimum 6 nodes
- ❌ Cross-slot operations limited

---

## Configuration

### Environment Variables

Storage configuration can be overridden with environment variables:

```bash
# Sled
export STATE_BACKEND=sled
export STATE_PATH=./data/sled

# Redis
export STATE_BACKEND=redis
export STATE_REDIS_URL=redis://localhost:6379/0

# Redis Cluster
export STATE_BACKEND=redis_cluster
export STATE_REDIS_CLUSTER_NODES=redis://node1:6379,redis://node2:6379
```

### Configuration File

Create a `config.toml` file (see `examples/config-*.toml` for complete examples):

```toml
[state]
backend = "sled"  # or "redis", "redis_cluster"
path = "./data/sled"  # For Sled
redis_url = "redis://localhost:6379/0"  # For Redis
pool_size = 100
```

### Programmatic Configuration

```rust
use llm_incident_manager::{
    config::{StateBackend, StateConfig},
    state::create_store,
};
use std::path::PathBuf;

// Sled configuration
let config = StateConfig {
    backend: StateBackend::Sled,
    path: Some(PathBuf::from("./data/sled")),
    redis_url: None,
    redis_cluster_nodes: vec![],
    pool_size: 100,
};

let store = create_store(&config).await?;
```

---

## Deployment Scenarios

### Scenario 1: Local Development

**Recommended**: In-Memory or Sled

**Configuration**:
```toml
[state]
backend = "sled"
path = "./data/dev"
```

**Rationale**: Fast startup, no external dependencies, automatic cleanup.

---

### Scenario 2: Single-Server Production

**Recommended**: Sled

**Configuration**:
```toml
[state]
backend = "sled"
path = "/var/lib/llm-incident-manager/data"
```

**Deployment**:
```bash
# Create data directory
mkdir -p /var/lib/llm-incident-manager/data
chown llm-im:llm-im /var/lib/llm-incident-manager/data
chmod 700 /var/lib/llm-incident-manager/data

# Run service
./llm-incident-manager --config /etc/llm-im/config.toml
```

**Backup**:
```bash
# Stop service
systemctl stop llm-incident-manager

# Backup database
tar -czf backup-$(date +%Y%m%d).tar.gz /var/lib/llm-incident-manager/data

# Restart service
systemctl start llm-incident-manager
```

---

### Scenario 3: Kubernetes Deployment

**Recommended**: Sled with PersistentVolume OR Redis

**Option A: Sled with StatefulSet**

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: llm-incident-manager
spec:
  serviceName: llm-im
  replicas: 1  # Single replica for Sled
  template:
    spec:
      containers:
      - name: llm-im
        image: llm-incident-manager:latest
        volumeMounts:
        - name: data
          mountPath: /data
        env:
        - name: STATE_BACKEND
          value: "sled"
        - name: STATE_PATH
          value: "/data/sled"
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 10Gi
```

**Option B: Redis**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-incident-manager
spec:
  replicas: 3  # Multiple replicas OK with Redis
  template:
    spec:
      containers:
      - name: llm-im
        image: llm-incident-manager:latest
        env:
        - name: STATE_BACKEND
          value: "redis"
        - name: STATE_REDIS_URL
          valueFrom:
            secretKeyRef:
              name: redis-credentials
              key: url
```

---

### Scenario 4: Multi-Region Deployment

**Recommended**: Redis Cluster with geographic distribution

**Architecture**:
```
Region US-East         Region US-West         Region EU-West
├─ Redis Master 1      ├─ Redis Master 2      ├─ Redis Master 3
├─ Redis Replica 2     ├─ Redis Replica 3     ├─ Redis Replica 1
├─ LLM-IM Nodes        ├─ LLM-IM Nodes        ├─ LLM-IM Nodes
```

**Configuration**:
```toml
[state]
backend = "redis_cluster"
redis_cluster_nodes = [
    "redis://us-east-master1:6379",
    "redis://us-west-master2:6379",
    "redis://eu-west-master3:6379",
]
```

---

## Performance Characteristics

### Throughput

| Backend | Read (ops/sec) | Write (ops/sec) | Notes |
|---------|----------------|-----------------|-------|
| In-Memory | 10M+ | 10M+ | Memory bandwidth limited |
| Sled | 500k | 100k | Disk I/O limited |
| Redis | 1M | 500k | Network and Redis config dependent |
| Redis Cluster | 10M | 5M | Scales horizontally |

### Latency (p50)

| Backend | Read | Write | Notes |
|---------|------|-------|-------|
| In-Memory | <1μs | <1μs | Nanosecond scale |
| Sled | <100μs | <500μs | With SSD |
| Redis | <1ms | <2ms | Local network |
| Redis Cluster | <2ms | <5ms | Includes cluster overhead |

### Storage Efficiency

| Backend | Overhead | Compression | Notes |
|---------|----------|-------------|-------|
| In-Memory | ~0% | No | Pure memory |
| Sled | ~20% | Yes | Write-ahead log + indexes |
| Redis | ~50% | Optional | Replication + indexes |
| Redis Cluster | ~50% | Optional | Sharding + replication |

### Resource Requirements

**Sled**:
- CPU: 1-2 cores
- RAM: 100MB + working set
- Disk: Incident data + 20% overhead
- IOPS: 1000+ recommended

**Redis**:
- CPU: 2-4 cores per Redis instance
- RAM: All data + 20% overhead
- Disk: AOF/RDB persistence
- Network: 1Gbps+

**Redis Cluster**:
- CPU: 2-4 cores per node × 6+ nodes
- RAM: Sharded data + 20% overhead per node
- Disk: AOF/RDB persistence per node
- Network: 10Gbps+ recommended

---

## Data Model

### Incident Storage

**Sled**:
```
incidents/{uuid} -> bincode(Incident)
fingerprints/{fingerprint} -> bincode(Vec<Uuid>)
```

**Redis**:
```
llm-im:incident:{uuid} -> json(Incident)
llm-im:incidents -> Set{uuid}
llm-im:severity:P0 -> Set{uuid}
llm-im:state:Active -> Set{uuid}
llm-im:source:{source} -> Set{uuid}
llm-im:fingerprint:{fingerprint} -> Set{uuid}
```

### Indexing

**Sled**:
- Primary key: UUID
- Secondary index: Fingerprint (manual)
- No automatic indexing

**Redis**:
- Primary key: UUID
- Automatic indexes: Severity, State, Source, Fingerprint
- Set-based intersections for complex queries

---

## Migration

### In-Memory to Sled

```rust
use llm_incident_manager::state::{InMemoryStore, SledStore, IncidentStore};

async fn migrate_to_sled() -> Result<()> {
    let source = InMemoryStore::new();
    let target = SledStore::new("./data/sled")?;

    // Get all incidents
    let filter = IncidentFilter::default();
    let incidents = source.list_incidents(&filter, 0, u32::MAX).await?;

    // Copy to Sled
    for incident in incidents {
        target.save_incident(&incident).await?;
    }

    target.flush().await?;
    Ok(())
}
```

### Sled to Redis

```rust
async fn migrate_to_redis() -> Result<()> {
    let source = SledStore::new("./data/sled")?;
    let target = RedisStore::new("redis://localhost:6379/0").await?;

    // Get all incidents
    let filter = IncidentFilter::default();
    let incidents = source.list_incidents(&filter, 0, u32::MAX).await?;

    // Copy to Redis
    for incident in incidents {
        target.save_incident(&incident).await?;
    }

    Ok(())
}
```

### Export to JSON

```rust
use std::fs::File;
use std::io::Write;

async fn export_to_json(store: &dyn IncidentStore, path: &str) -> Result<()> {
    let filter = IncidentFilter::default();
    let incidents = store.list_incidents(&filter, 0, u32::MAX).await?;

    let json = serde_json::to_string_pretty(&incidents)?;
    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;

    Ok(())
}
```

---

## Backup and Recovery

### Sled Backup

**Hot Backup** (online, no downtime):
```bash
# Create backup directory
mkdir -p /backups/$(date +%Y%m%d)

# Sled supports hot backups via copy
cp -r /var/lib/llm-incident-manager/data /backups/$(date +%Y%m%d)/
```

**Cold Backup** (offline, guaranteed consistency):
```bash
# Stop service
systemctl stop llm-incident-manager

# Create backup
tar -czf /backups/llm-im-$(date +%Y%m%d-%H%M%S).tar.gz \
    /var/lib/llm-incident-manager/data

# Restart service
systemctl start llm-incident-manager
```

**Recovery**:
```bash
# Stop service
systemctl stop llm-incident-manager

# Restore from backup
rm -rf /var/lib/llm-incident-manager/data
tar -xzf /backups/llm-im-20240115-120000.tar.gz \
    -C /var/lib/llm-incident-manager/

# Restart service
systemctl start llm-incident-manager
```

### Redis Backup

**RDB Snapshot**:
```bash
# Trigger save
redis-cli BGSAVE

# Copy RDB file
cp /var/lib/redis/dump.rdb /backups/dump-$(date +%Y%m%d).rdb
```

**AOF Backup**:
```bash
# Copy AOF file
cp /var/lib/redis/appendonly.aof /backups/appendonly-$(date +%Y%m%d).aof
```

**Recovery**:
```bash
# Stop Redis
systemctl stop redis

# Restore RDB
cp /backups/dump-20240115.rdb /var/lib/redis/dump.rdb
chown redis:redis /var/lib/redis/dump.rdb

# Restart Redis
systemctl start redis
```

---

## Troubleshooting

### Sled Issues

**Problem**: Database corruption after crash

**Solution**:
```bash
# Sled has automatic recovery, but if needed:
rm -rf /var/lib/llm-incident-manager/data/db.lock
# Restart service - Sled will recover from WAL
```

**Problem**: Disk space full

**Solution**:
```rust
// Check database size
let store = SledStore::new("./data")?;
let size = store.size_on_disk()?;
println!("Database size: {} bytes", size);

// Compact database (automatic, but can trigger manually)
store.flush().await?;
```

**Problem**: Slow queries

**Solution**:
- Ensure data directory is on SSD
- Increase page cache size (OS level)
- Monitor disk I/O with `iostat`

### Redis Issues

**Problem**: Connection refused

**Solution**:
```bash
# Check Redis is running
redis-cli ping

# Check Redis logs
tail -f /var/log/redis/redis.log

# Verify connection string
redis-cli -u redis://localhost:6379/0
```

**Problem**: Out of memory

**Solution**:
```bash
# Check memory usage
redis-cli INFO memory

# Check max memory config
redis-cli CONFIG GET maxmemory

# Set eviction policy
redis-cli CONFIG SET maxmemory-policy allkeys-lru
```

**Problem**: Slow queries

**Solution**:
```bash
# Enable slow log
redis-cli CONFIG SET slowlog-log-slower-than 10000

# Check slow queries
redis-cli SLOWLOG GET 10

# Monitor operations
redis-cli --latency
```

### General Issues

**Problem**: Data inconsistency

**Solution**:
```rust
// Verify incident count
let count = store.count_incidents(&IncidentFilter::default()).await?;
println!("Total incidents: {}", count);

// List all incidents and verify
let incidents = store.list_incidents(&filter, 0, u32::MAX).await?;
for incident in incidents {
    // Verify incident data integrity
    assert!(incident.id != Uuid::nil());
    assert!(!incident.title.is_empty());
}
```

**Problem**: Performance degradation

**Solution**:
1. Check resource utilization (CPU, RAM, Disk, Network)
2. Review logs for errors
3. Monitor query latency
4. Consider scaling (more replicas for Redis, bigger disk for Sled)

---

## Best Practices

### Development
- Use in-memory or Sled for local development
- Mount Sled data directory as volume for persistence
- Use separate Redis database (DB 15) for testing

### Production
- Always enable persistence (RDB+AOF for Redis)
- Configure automatic backups
- Monitor disk space and memory
- Set up alerts for storage errors
- Use connection pooling
- Enable compression for Redis

### Security
- Use TLS for Redis connections in production
- Restrict Redis network access (firewall, security groups)
- Use authentication (Redis requirepass or ACLs)
- Encrypt backups
- Regularly rotate credentials

### Monitoring
```bash
# Sled
- Database size on disk
- Flush duration
- Compaction frequency

# Redis
- Memory usage
- Connected clients
- Operations per second
- Slow queries
- Replication lag (if using replicas)
```

---

## Summary

| Requirement | Recommended Backend |
|-------------|-------------------|
| Development | In-Memory or Sled |
| Single server | Sled |
| Small scale (<100k incidents) | Sled |
| Medium scale (100k-1M incidents) | Redis |
| Large scale (>1M incidents) | Redis Cluster |
| High availability | Redis with Sentinel or Redis Cluster |
| Edge computing | Sled |
| Multi-region | Redis Cluster |
| Cost-sensitive | Sled |

All storage backends implement the same `IncidentStore` trait, making it easy to switch between them without code changes.
