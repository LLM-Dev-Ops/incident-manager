# Quick Crate Selection Guide for LLM-Incident-Manager

**Purpose:** Fast decision-making reference for selecting Rust crates

---

## Decision Trees

### Async Runtime Decision

```
Need async runtime?
├─ Yes → Use Tokio 1.48+
│  └─ Reason: Industry standard, best ecosystem
└─ Need minimal footprint?
   └─ Consider smol (but most crates expect Tokio)
```

**Winner:** `tokio = "1.48"`

---

### Message Queue Decision

```
Message Queue Type?
├─ RabbitMQ → lapin 3.7
│  └─ When: Complex routing, dead letters, guaranteed delivery
├─ Redis Pub/Sub → deadpool-redis 0.22
│  └─ When: Simple pub/sub, caching needs, lightweight
└─ In-Memory/Background Jobs → fang 0.13
   └─ When: Background processing, no external queue needed
```

**Recommendation:** Start with **RabbitMQ (lapin)** for production reliability

---

### Storage Decision

```
Storage Type?
├─ SQL Database
│  ├─ Need ORM? → Diesel 2.2
│  │  └─ When: Type-safe queries, schema management priority
│  └─ Need async + performance? → SQLx 0.8
│     └─ When: Async-first, raw SQL control, best performance
│
└─ Key-Value Store
   ├─ Production-ready? → RocksDB 0.23
   │  └─ When: Embedded storage, proven reliability
   └─ Pure Rust? → Wait for sled stable
      └─ Status: Beta, major rewrite in progress
```

**Recommendation:** **SQLx with PostgreSQL** for primary storage, **RocksDB** for caching

---

### Notification Channel Decision

```
Notification Type?
├─ Email → lettre 0.11
│  └─ Only mature Rust email library
│
├─ Slack
│  ├─ Simple webhooks? → slack-hook 0.8
│  └─ Full bot features? → slack-morphism 2.3
│
├─ Discord → webhook-rs 0.4
│
├─ Telegram → rustygram 0.2 (simple) or teloxide (full bot)
│
└─ Generic HTTP Webhooks → reqwest 0.12
   └─ Use for all custom webhook integrations
```

**Required:** `lettre`, `reqwest`
**Optional:** Add Slack/Discord/Telegram as needed

---

### Scheduling Decision

```
Scheduling Needs?
├─ Complex scheduling + persistence
│  └─ tokio-cron-scheduler 0.15
│     └─ Features: PostgreSQL/Nats persistence, job notifications
│
├─ Just cron parsing
│  ├─ Advanced features (L, #, W) → croner 2.0
│  └─ Simple parsing → cron 0.12
│
└─ Background job queue
   └─ fang 0.13
      └─ Features: Retries, async workers, CRON tasks
```

**Recommendation:** **tokio-cron-scheduler** for scheduling, **fang** for background jobs

---

### Serialization Decision

```
Serialization Format?
├─ Internal/Performance Critical → bincode 2.0
│  └─ Fastest, most compact
│
├─ API/External → serde_json 1.0
│  └─ Human-readable, widely supported
│
├─ Multi-language → rmp-serde 1.3 (MessagePack)
│  └─ Compact + language-agnostic
│
└─ Configuration
   ├─ TOML → toml 0.8
   └─ YAML → serde_yaml 0.9
```

**Required:** `serde`, `serde_json`, `bincode`

---

### Logging Decision

```
Logging Strategy?
├─ Building a library? → log facade
│  └─ Maximum compatibility
│
├─ Building an application? → tracing 0.1
│  └─ Structured logging, async support, modern
│
└─ Need observability?
   └─ tracing + OpenTelemetry 0.27
      └─ Distributed tracing, metrics, production observability
```

**Recommendation:** **tracing** + **OpenTelemetry** for production applications

---

### Metrics Decision

```
Metrics Collection?
├─ Prometheus → prometheus 0.13 (TikV) or prometheus-client 0.22 (official)
│  └─ Industry standard, Grafana integration
│
├─ OpenTelemetry Metrics → opentelemetry 0.27
│  └─ Vendor-agnostic, OTLP export
│
└─ Both
   └─ Use OpenTelemetry, export to Prometheus
```

**Recommendation:** **prometheus** (TikV) for simplicity, **OpenTelemetry** for full observability

---

### Error Handling Decision

```
Error Handling Context?
├─ Library code → thiserror 2.0
│  └─ Custom error types, clear error definitions
│
└─ Application code → anyhow 2.0
   └─ Rich context, easy propagation
```

**Required:** Both - use `thiserror` for custom types, `anyhow` for main app

---

## Quick Reference: Must-Have Crates

### Core Dependencies (Always Include)

```toml
tokio = { version = "1.48", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
anyhow = "2.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

### Communication Layer

```toml
lapin = "3.7"                              # RabbitMQ
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
lettre = { version = "0.11", features = ["tokio1-rustls-tls", "smtp-transport"] }
```

### Storage Layer

```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres"] }
deadpool-redis = "0.22"
```

### Scheduling & Jobs

```toml
tokio-cron-scheduler = { version = "0.15", features = ["postgres_storage"] }
fang = { version = "0.13", features = ["asynk", "postgres"] }
```

### Observability

```toml
opentelemetry = { version = "0.27", features = ["metrics", "trace"] }
opentelemetry-otlp = "0.27"
prometheus = "0.13"
```

---

## Performance Optimization Checklist

### High Priority

- [ ] Use `bincode` for internal serialization (not JSON)
- [ ] Enable connection pooling (sqlx::Pool, deadpool-redis)
- [ ] Use `tokio` with `tracing` feature for debugging
- [ ] Enable LTO in release profile
- [ ] Use `rustls` instead of `native-tls` (smaller, faster)

### Medium Priority

- [ ] Implement proper error handling (thiserror + anyhow)
- [ ] Add structured logging with tracing
- [ ] Set up Prometheus metrics
- [ ] Configure log levels via environment (RUST_LOG)

### Low Priority (Once Stable)

- [ ] Profile with flamegraph
- [ ] Benchmark critical paths with criterion
- [ ] Monitor with tokio-console
- [ ] Optimize database queries

---

## Common Patterns

### Pattern 1: Notification Trait

```rust
#[async_trait]
pub trait NotificationChannel {
    async fn send(&self, incident: &Incident) -> Result<()>;
}

// Implement for Email, Slack, Webhooks, etc.
```

### Pattern 2: Message Queue Abstraction

```rust
#[async_trait]
pub trait MessageQueue {
    async fn publish(&self, event: Event) -> Result<()>;
    async fn subscribe(&self, queue: &str) -> Result<Receiver<Event>>;
}

// Implement for RabbitMQ, Redis, etc.
```

### Pattern 3: Storage Repository

```rust
#[async_trait]
pub trait IncidentRepository {
    async fn save(&self, incident: &Incident) -> Result<()>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Incident>>;
    async fn list_active(&self) -> Result<Vec<Incident>>;
}

// Implement for PostgreSQL, RocksDB, etc.
```

### Pattern 4: Error Context

```rust
use anyhow::{Context, Result};

async fn process_incident(id: Uuid) -> Result<()> {
    let incident = repo.find_by_id(id)
        .await
        .context(format!("Failed to fetch incident {}", id))?;

    notify(&incident)
        .await
        .context("Failed to send notification")?;

    Ok(())
}
```

---

## Testing Strategy

### Unit Tests

```toml
[dev-dependencies]
tokio-test = "0.4"
mockall = "0.13"
```

### Integration Tests

```toml
[dev-dependencies]
testcontainers = "0.23"  # Docker containers for tests
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "macros"] }
```

### Benchmarks

```toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "serialization"
harness = false
```

---

## Migration Path

### Phase 1: Core (Week 1-2)

1. Set up Tokio runtime
2. Implement basic error handling (thiserror + anyhow)
3. Add tracing logging
4. Set up PostgreSQL with SQLx

### Phase 2: Messaging (Week 3-4)

1. Integrate RabbitMQ (lapin)
2. Implement notification channels (lettre, reqwest)
3. Add Redis caching (deadpool-redis)

### Phase 3: Scheduling (Week 5-6)

1. Set up tokio-cron-scheduler
2. Implement background jobs (fang)
3. Add monitoring checks

### Phase 4: Observability (Week 7-8)

1. Add OpenTelemetry integration
2. Set up Prometheus metrics
3. Configure distributed tracing

---

## Version Compatibility Matrix

| Crate | Version | Tokio | MSRV |
|-------|---------|-------|------|
| tokio | 1.48 | - | 1.70 |
| lapin | 3.7 | 1.x | 1.75 |
| sqlx | 0.8 | 1.x | 1.75 |
| lettre | 0.11 | 1.x | 1.74 |
| reqwest | 0.12 | 1.x | 1.70 |
| serde | 1.0.228 | - | 1.60 |
| tracing | 0.1 | 1.x | 1.63 |

**Project MSRV:** Rust 1.75+ (to support all crates)

---

## Security Best Practices

### Dependency Management

```bash
# Check for security vulnerabilities
cargo audit

# Update dependencies
cargo update

# Check for outdated crates
cargo outdated
```

### Configuration

```rust
// Use environment variables for secrets
use dotenv::dotenv;
use std::env;

dotenv().ok();
let db_url = env::var("DATABASE_URL")
    .expect("DATABASE_URL must be set");
```

### TLS Configuration

```toml
# Always use TLS in production
lettre = { version = "0.11", features = ["tokio1-rustls-tls"] }
reqwest = { version = "0.12", features = ["rustls-tls"] }
```

---

## Troubleshooting

### Common Issues

**Issue:** Tokio runtime not available
```rust
// Solution: Wrap async code in tokio::main
#[tokio::main]
async fn main() {
    // Your async code
}
```

**Issue:** Connection pool exhausted
```rust
// Solution: Configure pool size
let pool = PgPoolOptions::new()
    .max_connections(50)
    .connect(&database_url).await?;
```

**Issue:** Slow compilation
```rust
// Solution: Reduce feature flags, use sccache
// In .cargo/config.toml
[build]
rustc-wrapper = "sccache"
```

**Issue:** Large binary size
```toml
# Solution: Strip symbols, enable LTO
[profile.release]
strip = true
lto = true
```

---

## Resources

### Official Documentation

- Tokio: https://tokio.rs
- SQLx: https://github.com/launchbadge/sqlx
- Serde: https://serde.rs
- Tracing: https://docs.rs/tracing

### Learning Resources

- Async Rust Book: https://rust-lang.github.io/async-book/
- Tokio Tutorial: https://tokio.rs/tokio/tutorial
- Rust by Example: https://doc.rust-lang.org/rust-by-example/

### Community

- Rust Users Forum: https://users.rust-lang.org
- Tokio Discord: https://discord.gg/tokio
- r/rust: https://reddit.com/r/rust

---

**Document Version:** 1.0
**Last Updated:** November 11, 2025
