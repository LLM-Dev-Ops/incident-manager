# LLM Incident Manager - Technical Architecture Decisions

## Overview
This document captures key technical decisions made during the planning phase of the LLM Incident Manager project. Each decision includes context, alternatives considered, rationale, and implications.

---

## Architecture Decisions

### ADR-001: Microservices vs Monolithic Architecture

**Status**: DECIDED - Start with Modular Monolith

**Context**:
The system needs to handle incident ingestion, classification, routing, and notifications with potential for high scalability.

**Decision**:
Implement a modular monolithic architecture for MVP and Beta, with clear service boundaries that can be extracted into microservices for v1.0 if needed.

**Rationale**:
- Faster development velocity for MVP/Beta
- Simpler deployment and debugging
- Easier to refactor with real usage data
- Can extract hot paths to microservices in v1.0
- Reduces operational complexity initially

**Alternatives Considered**:
1. **Microservices from Day 1**: Too much overhead for MVP, premature optimization
2. **Pure Monolith**: Harder to scale specific components later
3. **Serverless**: Cold start issues for time-sensitive incident processing

**Implications**:
- Need clear module boundaries (ingestion, classification, routing, persistence)
- Plan for service extraction in v1.0 phase
- Use message queues internally to prepare for distributed architecture

**Review Date**: Week 12 (Beta completion)

---

### ADR-002: Database Selection - PostgreSQL

**Status**: DECIDED

**Context**:
Need persistent storage for incidents, metadata, configuration, and historical data with support for complex queries and transactions.

**Decision**:
Use PostgreSQL as primary database with pgvector extension for similarity search (deduplication).

**Rationale**:
- Excellent JSON/JSONB support for flexible incident schemas
- ACID compliance for critical incident data
- pgvector extension for vector similarity (deduplication)
- Mature replication and HA capabilities
- Strong ecosystem and tooling
- Time-series capabilities with TimescaleDB extension (future)

**Alternatives Considered**:
1. **MongoDB**: Less mature transaction support, harder to maintain consistency
2. **MySQL**: Weaker JSON support, no native vector search
3. **DynamoDB**: Vendor lock-in, complex querying, cost concerns
4. **CockroachDB**: Overkill for MVP, adds complexity

**Implications**:
- Need to design schema carefully for flexible JSON storage
- Plan for partitioning strategy (by month) early
- Use connection pooling (PgBouncer) for scale
- Consider read replicas for Beta phase

**Performance Targets**:
- Single incident read: < 100ms
- List query (100 items): < 500ms
- Write latency: < 50ms

---

### ADR-003: LLM Provider - Claude (Primary)

**Status**: DECIDED

**Context**:
Core classification, deduplication, and AI features require LLM capabilities.

**Decision**:
Use Claude (Anthropic) as primary LLM provider with fallback to rule-based classification.

**Rationale**:
- Superior reasoning capabilities for incident classification
- Longer context window (200K tokens) for rich incident data
- Built-in safety features
- Project already uses claude-flow framework
- Good API reliability and rate limits

**Alternatives Considered**:
1. **OpenAI GPT-4**: Strong alternative, consider as backup
2. **Open-source LLMs (Llama, Mistral)**: Self-hosting complexity, accuracy concerns
3. **Multiple providers**: Adds complexity, consider for v1.0

**Fallback Strategy**:
- Rule-based classifier using keywords, severity keywords, source mapping
- Cache classification results (1 hour TTL)
- Queue requests with exponential backoff on rate limits

**Cost Management**:
- Estimated $0.002-0.005 per incident classification
- Implement request batching where possible
- Use smaller models for simple classifications

**Review Date**: Week 8 (evaluate accuracy and cost)

---

### ADR-004: API Framework - Node.js with Fastify

**Status**: DECIDED

**Context**:
Need high-performance API server for incident ingestion and query endpoints.

**Decision**:
Use Node.js with Fastify framework.

**Rationale**:
- Fastify provides excellent performance (20K req/s+)
- Schema validation built-in (JSON Schema)
- Plugin ecosystem for common needs
- Better performance than Express
- Async/await support for LLM and database calls
- Good TypeScript support

**Alternatives Considered**:
1. **Express**: Slower, less modern, but more familiar
2. **Go**: Faster but steeper learning curve, smaller team pool
3. **Python (FastAPI)**: Good for ML integration but slower runtime
4. **Rust (Actix)**: Excellent performance but limited team expertise

**Implications**:
- Use TypeScript for type safety
- Implement request validation with JSON Schema
- Use clustering for multi-core utilization
- Plan for horizontal scaling with load balancer

---

### ADR-005: Frontend Framework - React with Next.js

**Status**: DECIDED

**Context**:
Need responsive, real-time dashboard for incident management.

**Decision**:
Use React with Next.js framework for the dashboard.

**Rationale**:
- React is industry standard with large talent pool
- Next.js provides SSR, API routes, and optimization
- Excellent developer experience
- Strong ecosystem for UI components (Shadcn/ui, MUI)
- WebSocket support for real-time updates

**Alternatives Considered**:
1. **Vue.js**: Smaller ecosystem, team less familiar
2. **Svelte**: Less mature, smaller talent pool
3. **Angular**: More opinionated, steeper learning curve
4. **Plain React (CRA)**: Less optimized, no SSR

**Technical Stack**:
- **State Management**: Zustand or React Query
- **UI Components**: Shadcn/ui + Tailwind CSS
- **Real-time**: Socket.io or WebSocket API
- **Charts**: Recharts or Chart.js
- **Forms**: React Hook Form + Zod validation

---

### ADR-006: Message Queue - Kafka for v1.0, In-Memory for MVP

**Status**: DECIDED

**Context**:
Need reliable message queuing for asynchronous processing and event distribution.

**Decision**:
- **MVP/Beta**: In-memory queue (Bull with Redis backing)
- **v1.0**: Migrate to Apache Kafka for HA and scale

**Rationale**:
- MVP doesn't need full Kafka complexity
- Bull provides good developer experience with Redis
- Kafka in v1.0 for multi-region replication and scale
- Gradual migration path

**Alternatives Considered**:
1. **RabbitMQ**: Good fit but Kafka better for high throughput
2. **AWS SQS**: Vendor lock-in, higher latency
3. **Redis Streams**: Less mature, no strong guarantees
4. **Kafka from Day 1**: Overkill for MVP

**Migration Plan**:
- Design queue interfaces to be pluggable
- Implement adapter pattern for easy swap
- Migrate critical paths first (ingestion → classification)

---

### ADR-007: Caching Strategy - Redis

**Status**: DECIDED

**Context**:
Need caching for classification results, session data, and rate limiting.

**Decision**:
Use Redis for all caching needs, starting Beta phase.

**Rationale**:
- Industry standard for caching
- Built-in data structures (sets, sorted sets, hashes)
- Pub/sub for real-time features
- Can be used for session storage
- Easy to cluster for HA

**Cache Layers**:
1. **Classification Results**: 1-hour TTL
2. **API Responses**: 5-minute TTL
3. **User Sessions**: 24-hour TTL
4. **Rate Limiting**: Sliding window (1 minute)
5. **Dashboard Aggregates**: 5-minute TTL

**Alternatives Considered**:
1. **Memcached**: Simpler but fewer features
2. **In-Memory (Node)**: Not shared across instances
3. **Database Caching**: Slower, adds DB load

---

### ADR-008: Authentication - JWT with API Keys

**Status**: DECIDED

**Context**:
Need secure authentication for API and dashboard access.

**Decision**:
- **API**: API keys with scoped permissions
- **Dashboard**: JWT tokens with refresh tokens
- **v1.0**: Integrate with Identity Service (SSO)

**Rationale**:
- API keys good for programmatic access
- JWT stateless and scalable
- Prepare for SSO integration in v1.0

**Security Measures**:
- API keys hashed in database (bcrypt)
- JWT signed with RS256 (asymmetric)
- Refresh token rotation
- Rate limiting per API key
- IP allowlisting (optional)

**Alternatives Considered**:
1. **OAuth 2.0**: Too complex for MVP
2. **Session-based**: Not scalable for multi-instance
3. **Basic Auth**: Less secure, no scoping

---

### ADR-009: Real-Time Updates - WebSocket

**Status**: DECIDED

**Context**:
Dashboard needs real-time incident updates without polling.

**Decision**:
Use WebSocket (Socket.io) for real-time updates starting Beta phase.

**Rationale**:
- True bidirectional real-time communication
- Socket.io handles reconnection and fallbacks
- Can broadcast to multiple clients efficiently
- Good Node.js integration

**Implementation**:
- Server-side: Socket.io with Redis adapter (multi-instance)
- Client-side: Socket.io client with auto-reconnect
- Events: incident.created, incident.updated, incident.resolved
- Rooms: Per-user, per-team, global

**Alternatives Considered**:
1. **Polling**: Simple but inefficient, 30s delay
2. **Server-Sent Events (SSE)**: One-way only
3. **GraphQL Subscriptions**: Adds complexity

**Fallback**: Polling every 30 seconds if WebSocket unavailable

---

### ADR-010: Deployment - Kubernetes

**Status**: DECIDED

**Context**:
Need scalable, resilient deployment platform for v1.0 HA requirements.

**Decision**:
- **MVP**: Docker containers on VMs (simple deployment)
- **Beta**: Single Kubernetes cluster
- **v1.0**: Multi-region Kubernetes with service mesh

**Rationale**:
- Kubernetes is industry standard for container orchestration
- Auto-scaling and self-healing
- Service discovery and load balancing
- Multi-region support for HA
- Mature ecosystem (Helm, Istio, etc.)

**Infrastructure as Code**:
- **IaC**: Terraform for cloud resources
- **K8s Manifests**: Helm charts for application deployment
- **GitOps**: ArgoCD or Flux for continuous delivery

**Alternatives Considered**:
1. **Docker Swarm**: Less mature, smaller ecosystem
2. **ECS (AWS)**: Vendor lock-in
3. **Serverless (Lambda/Cloud Run)**: Cold start issues
4. **VMs only**: Manual scaling, no self-healing

---

### ADR-011: Observability Stack

**Status**: DECIDED

**Context**:
Need comprehensive monitoring, logging, and tracing for production reliability.

**Decision**:
Use industry-standard observability stack:
- **Metrics**: Prometheus + Grafana
- **Logs**: Loki + Grafana (or ELK stack)
- **Traces**: Jaeger (OpenTelemetry)
- **Alerts**: Alertmanager + PagerDuty

**Rationale**:
- Open-source and vendor-neutral
- Strong Kubernetes integration
- Unified visualization with Grafana
- OpenTelemetry for future flexibility

**Key Metrics**:
- RED (Rate, Errors, Duration) for services
- USE (Utilization, Saturation, Errors) for resources
- Business metrics (incidents/min, classification accuracy)

**Alternatives Considered**:
1. **Datadog**: Expensive, vendor lock-in
2. **New Relic**: Good but costly at scale
3. **ELK Stack**: Good for logs but separate for metrics
4. **Cloud-native (CloudWatch)**: Vendor lock-in

**Integration Timeline**: Week 17-18 (v1.0 phase)

---

### ADR-012: Testing Strategy

**Status**: DECIDED

**Context**:
Need comprehensive testing to ensure reliability and prevent regressions.

**Decision**:
Multi-layered testing approach:
1. **Unit Tests**: Jest (90% coverage target)
2. **Integration Tests**: Supertest + Testcontainers (85% coverage)
3. **E2E Tests**: Playwright (70% coverage)
4. **Load Tests**: k6 (2x capacity validation)
5. **Chaos Tests**: Chaos Mesh (v1.0 resilience)

**Test Pyramid**:
```
        /\
       /E2E\       (70% coverage - critical paths)
      /------\
     /  INT   \    (85% coverage - API + integrations)
    /----------\
   /    UNIT    \  (90% coverage - business logic)
  /--------------\
```

**CI/CD Pipeline**:
- Unit tests on every commit (< 2 min)
- Integration tests on PR (< 10 min)
- E2E tests on merge to main (< 30 min)
- Load tests weekly and before releases
- Chaos tests in staging before v1.0 release

**Quality Gates**:
- All tests must pass to merge
- Coverage must not decrease
- Performance benchmarks must be met

---

### ADR-013: Data Retention and Archival

**Status**: DECIDED

**Context**:
Need to manage data growth while maintaining historical visibility and compliance.

**Decision**:
Tiered data retention strategy:
- **Hot Data** (0-90 days): Full access in primary database
- **Warm Data** (91-365 days): Archived to object storage, queryable
- **Cold Data** (1+ years): Compressed archive, restore on demand
- **Compliance Data**: 7-year retention in immutable storage

**Implementation**:
- Daily archival job (runs at 2 AM)
- PostgreSQL partitioning by month
- S3/GCS for archived data
- Restore API for historical queries

**Compliance**:
- GDPR: Right to deletion (mark deleted, purge after 30 days)
- SOC2: Immutable audit logs (7 years)
- Data export: JSON/CSV export API

**Alternatives Considered**:
1. **Keep everything in DB**: Cost prohibitive, slow queries
2. **Delete after 90 days**: Loses historical insights
3. **External analytics DB**: Added complexity

---

### ADR-014: Multi-Tenancy (Future)

**Status**: PROPOSED - Post v1.0

**Context**:
Future enterprise customers may need multi-tenancy for organizational isolation.

**Proposal**:
- **v1.0**: Single-tenant deployments
- **v1.2**: Shared infrastructure, logical isolation (tenant_id in all tables)
- **v2.0**: Full multi-tenancy with tenant-specific configs

**Design Considerations**:
- Row-level security (RLS) in PostgreSQL
- Tenant-aware caching
- Separate API keys per tenant
- Tenant-specific rate limits
- Data isolation guarantees

**Not Deciding Now**: Defer to post-v1.0 based on customer demand

---

## Technology Stack Summary

### Backend
- **Runtime**: Node.js 20 LTS
- **Framework**: Fastify 4.x
- **Language**: TypeScript 5.x
- **Database**: PostgreSQL 15+ with pgvector
- **Cache**: Redis 7.x
- **Message Queue**: Bull (MVP/Beta), Kafka (v1.0)
- **LLM**: Claude API (Anthropic)

### Frontend
- **Framework**: React 18 + Next.js 14
- **Language**: TypeScript
- **State**: Zustand + React Query
- **UI**: Shadcn/ui + Tailwind CSS
- **Real-time**: Socket.io
- **Charts**: Recharts

### Mobile
- **Framework**: React Native
- **Platform**: iOS + Android
- **State**: Zustand
- **UI**: React Native Paper

### Infrastructure
- **Containers**: Docker
- **Orchestration**: Kubernetes 1.28+
- **IaC**: Terraform
- **GitOps**: ArgoCD
- **Service Mesh**: Istio (v1.0)

### Observability
- **Metrics**: Prometheus + Grafana
- **Logs**: Loki + Grafana
- **Traces**: Jaeger (OpenTelemetry)
- **Alerts**: Alertmanager

### CI/CD
- **Source Control**: Git (GitHub/GitLab)
- **CI**: GitHub Actions or GitLab CI
- **CD**: ArgoCD (Kubernetes)
- **Artifact Registry**: Docker Hub or GHCR

---

## Security Considerations

### Application Security
- Input validation on all API endpoints (JSON Schema)
- SQL injection prevention (parameterized queries)
- XSS prevention (React auto-escaping)
- CSRF protection (tokens)
- Rate limiting (per IP, per API key)
- Secrets management (Vault or cloud KMS)

### Infrastructure Security
- TLS 1.3 for all communications
- Network policies (K8s NetworkPolicy)
- Pod security policies
- Least privilege IAM roles
- Regular security scanning (Trivy, Snyk)
- Vulnerability patching SLA (24h for critical)

### Compliance
- Encryption at rest (AES-256)
- Encryption in transit (TLS 1.3)
- Audit logging (all data access)
- Data retention policies
- GDPR compliance (right to delete, export)
- SOC2 Type II (v1.0 goal)

---

## Performance Targets

### MVP (v0.1)
- Ingestion: 10 incidents/min
- Classification: < 2s (p95)
- Notification: < 5s end-to-end (p95)
- API Response: < 200ms (p95)
- Dashboard Load: < 2s

### Beta (v0.5)
- Ingestion: 100 incidents/min
- Classification: < 2s (p95)
- Notification: < 3s end-to-end (p95)
- API Response: < 150ms (p95)
- Dashboard Load: < 1.5s

### v1.0
- Ingestion: 500+ incidents/min
- Classification: < 2s (p99)
- Notification: < 5s end-to-end (p99)
- API Response: < 200ms (p99)
- Dashboard Load: < 1s
- Real-time Update: < 1s latency

---

## Cost Optimization Strategies

### Development Phase
- Use shared development environments
- Leverage free tiers (Postgres, Redis on Render/Railway)
- Local Kubernetes (minikube, kind) for development

### Production
- Right-size instances based on metrics
- Auto-scaling with aggressive scale-down
- Spot instances for non-critical workloads
- Reserved instances for baseline capacity
- CDN for static assets (CloudFlare free tier)
- Database read replicas only where needed
- Cache aggressively to reduce DB load and LLM calls

### LLM Cost Optimization
- Batch classification requests where possible
- Use smaller models for simple classifications
- Cache classification results (1-hour TTL)
- Implement confidence thresholds to skip LLM for obvious cases
- Rate limiting to prevent abuse

**Target**: < $0.01 per incident processed (all costs)

---

## Decision Review Schedule

| Decision | Review Date | Owner | Status |
|----------|-------------|-------|--------|
| ADR-001: Architecture | Week 12 | Lead Architect | Pending |
| ADR-003: LLM Provider | Week 8 | Backend Lead | Pending |
| ADR-006: Message Queue | Week 14 | Infrastructure Lead | Pending |
| ADR-010: Deployment | Week 16 | DevOps Lead | Pending |
| All ADRs | Week 20 (v1.0) | TPM | Scheduled |

---

## Change Log

| Date | Version | Changes | Author |
|------|---------|---------|--------|
| 2025-11-11 | 1.0 | Initial technical decisions | TPM |

---

**Document Status**: APPROVED for MVP Planning
**Last Updated**: 2025-11-11
**Owner**: Technical Program Manager + Lead Architect
**Reviewers**: Engineering team, Security team, DevOps team
