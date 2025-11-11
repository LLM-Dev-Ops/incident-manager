# LLM Incident Manager - Quick Reference Guide

## Project at a Glance

**Project**: LLM Incident Manager
**Vision**: Intelligent, automated incident management powered by LLM technology
**Timeline**: 20 weeks (5 months)
**Team Size**: 3.5 → 5.5 FTE (phased)
**Budget**: ~$1,100/mo (MVP) → ~$13,000/mo (v1.0) infrastructure

---

## Three-Phase Roadmap

### Phase 1: MVP (v0.1) - Weeks 1-8
**Core Functionality**: Single-channel incident management

✓ HTTP webhook ingestion
✓ LLM-powered classification
✓ Slack notifications
✓ PostgreSQL persistence
✓ REST API
✓ Basic React dashboard
✓ Monitoring & logging

**Key Metric**: Process 10 incidents/min with 85%+ classification accuracy

---

### Phase 2: Beta (v0.5) - Weeks 9-14
**Enhanced Features**: Multi-channel, intelligent deduplication

✓ 4+ notification channels (Slack, PagerDuty, Teams, Email)
✓ LLM-based deduplication (pgvector)
✓ Escalation policies
✓ Incident assignment
✓ Redis caching
✓ GraphQL API
✓ Enhanced dashboard (WebSocket updates)

**Key Metric**: Process 100 incidents/min, 30%+ noise reduction

---

### Phase 3: v1.0 - Weeks 15-20
**Production-Ready**: HA, AI features, compliance

✓ Multi-region HA architecture
✓ Advanced AI (RCA, prediction, NL queries)
✓ Compliance integration
✓ Comprehensive observability
✓ Workflow engine
✓ Mobile apps (iOS/Android)
✓ Load & chaos testing

**Key Metric**: 99.95% uptime, 500+ incidents/min, 4.5/5 satisfaction

---

## Technology Stack

### Core Technologies
| Component | Technology | Why |
|-----------|-----------|-----|
| **Backend** | Node.js + Fastify | High performance, async-friendly |
| **Frontend** | React + Next.js | Industry standard, SSR support |
| **Database** | PostgreSQL + pgvector | ACID, JSON, vector search |
| **Cache** | Redis | Industry standard, pub/sub |
| **LLM** | Claude (Anthropic) | Superior reasoning, long context |
| **Queue** | Bull → Kafka | Simple to start, scales later |
| **Deploy** | Docker → Kubernetes | Container standard, orchestration |

### Observability
- **Metrics**: Prometheus + Grafana
- **Logs**: Loki + Grafana
- **Traces**: Jaeger (OpenTelemetry)
- **Alerts**: Alertmanager + PagerDuty

---

## Key Features by Phase

```
Feature                    │ MVP │ Beta │ v1.0
═══════════════════════════╪═════╪══════╪═════
Incident Ingestion         │  ✓  │  ✓   │  ✓
LLM Classification         │  ✓  │  ✓   │  ✓
Single Channel (Slack)     │  ✓  │  ✓   │  ✓
Multi-Channel (4+)         │  ✗  │  ✓   │  ✓
Deduplication              │  ✗  │  ✓   │  ✓
Escalation Policies        │  ✗  │  ✓   │  ✓
Assignment & Ownership     │  ✗  │  ✓   │  ✓
REST API                   │  ✓  │  ✓   │  ✓
GraphQL API                │  ✗  │  ✓   │  ✓
Dashboard (Basic)          │  ✓  │  ✓   │  ✓
Dashboard (Real-time)      │  ✗  │  ✓   │  ✓
High Availability          │  ✗  │  ✗   │  ✓
Multi-Region               │  ✗  │  ✗   │  ✓
AI Features (RCA, NL)      │  ✗  │  ✗   │  ✓
Compliance Integration     │  ✗  │  ✗   │  ✓
Workflow Engine            │  ✗  │  ✗   │  ✓
Mobile Apps                │  ✗  │  ✗   │  ✓
Observability Stack        │  ✗  │  ✗   │  ✓
```

---

## Performance Targets

### Throughput
| Phase | Incidents/Min | Concurrent Users | Active Incidents |
|-------|---------------|------------------|------------------|
| MVP   | 10            | 10               | 1,000            |
| Beta  | 100           | 100              | 10,000           |
| v1.0  | 500+          | 1,000+           | 100,000+         |

### Latency (p95)
| Operation | MVP | Beta | v1.0 |
|-----------|-----|------|------|
| Ingestion | 200ms | 150ms | 200ms |
| Classification | 2s | 2s | 2s (p99) |
| End-to-End | 5s | 3s | 5s (p99) |
| API Call | 200ms | 150ms | 200ms (p99) |
| Dashboard Load | 2s | 1.5s | 1s |

### Reliability
| Metric | MVP | Beta | v1.0 |
|--------|-----|------|------|
| Uptime | 99% | 99.5% | 99.95% |
| Data Durability | 99.9% | 99.99% | 99.999999999% |
| Classification Accuracy | 85% | 90% | 85%+ |
| Notification Delivery | 99% | 99.5% | 99.9% |

---

## Team Structure

### MVP (Weeks 1-8): 3.5 FTE
- 2x Backend Engineers
- 1x Full-Stack Engineer
- 0.5x DevOps Engineer

### Beta (Weeks 9-14): 4.25 FTE
- 2x Backend Engineers
- 1x Full-Stack Engineer
- 0.75x DevOps Engineer
- 0.5x QA Engineer

### v1.0 (Weeks 15-20): 5.5 FTE
- 2x Backend Engineers
- 1x Full-Stack Engineer
- 1x DevOps/SRE Engineer
- 1x QA Engineer
- 0.5x Technical Writer

---

## Critical Dependencies

### External LLM DevOps Modules

| Module | Phase | Criticality | Fallback |
|--------|-------|-------------|----------|
| Identity Service | v1.0 | HIGH | Built-in JWT auth |
| Compliance Module | v1.0 | HIGH | Local audit logs |
| Observability Module | v1.0 | MEDIUM | Prometheus + Grafana |
| Knowledge Base | v1.0 | MEDIUM | Local runbook storage |
| Cost Tracker | v1.0 | LOW | Basic cost tracking |

### Third-Party Services

| Service | Phase | Purpose | Fallback |
|---------|-------|---------|----------|
| Claude API | MVP | Classification | Rule-based classifier |
| Slack API | MVP | Notifications | Email (SMTP) |
| PagerDuty API | Beta | Critical alerts | Phone calls (Twilio) |
| Redis | Beta | Caching | In-memory cache |
| PostgreSQL | MVP | Persistence | SQLite (dev/test) |

---

## Testing Strategy Summary

### Test Coverage Targets
| Test Type | MVP | Beta | v1.0 |
|-----------|-----|------|------|
| Unit | 80% | 85% | 90% |
| Integration | 60% | 75% | 85% |
| E2E | 20% | 50% | 70% |
| Load | Basic | Moderate | 2x capacity |
| Chaos | N/A | Basic | Comprehensive |
| Security | Basic | Medium | Full audit |

### Test Tools
- **Unit**: Jest + Supertest
- **Integration**: Testcontainers
- **E2E**: Playwright (web), Appium (mobile)
- **Load**: k6
- **Chaos**: Chaos Mesh
- **Security**: Snyk, Trivy, OWASP ZAP

---

## Risk Heat Map

### High Priority (Immediate Mitigation Required)
1. **LLM API Rate Limits/Outages**: Queue + fallback classifier + caching
2. **Third-Party Module Delays**: Standalone features + adapter pattern
3. **Scale/Performance Issues**: Early load testing + horizontal scaling
4. **Data Loss**: Replication + backups + transaction management
5. **Security Vulnerabilities**: Regular scanning + pen testing + training

### Medium Priority (Monitor & Plan)
6. **Classification Accuracy**: Prompt tuning + feedback loop + testing
7. **Integration Complexity**: Phased approach + testing + rollback
8. **User Adoption**: Documentation + training + support
9. **Cost Overruns**: Monitoring + optimization + alerts
10. **Team Turnover**: Documentation + cross-training + knowledge sharing

---

## Key Milestones & Decision Points

```
Week 4:  LLM Provider Selection (Claude vs. Multi-provider)
Week 8:  MVP Go/No-Go Decision
Week 10: Beta Channel Selection (4 vs. 6+ channels)
Week 14: Beta Go/No-Go Decision
Week 16: HA Scope (2-region vs. 3-region)
Week 18: AI Features Scope (Core vs. Full suite)
Week 20: v1.0 Go/No-Go Decision
```

**Escalation Path**: TPM → Engineering Manager → VP Engineering

---

## Success Criteria Checklist

### MVP (Week 8) - 8/10 Required
- [ ] 99.5%+ ingestion success rate
- [ ] 85%+ classification accuracy
- [ ] < 5s end-to-end latency (p95)
- [ ] 1,000+ incidents stored
- [ ] API fully documented
- [ ] Dashboard functional
- [ ] 80%+ test coverage
- [ ] Zero critical bugs
- [ ] End-to-end demo complete
- [ ] Monitoring operational

### Beta (Week 14) - 9/11 Required
- [ ] 4+ notification channels
- [ ] 30%+ noise reduction via deduplication
- [ ] Escalation policies accurate
- [ ] Assignment system working
- [ ] Real-time dashboard updates
- [ ] 10,000+ incidents supported
- [ ] 100 incidents/min sustained
- [ ] 10+ beta users onboarded
- [ ] 85%+ test coverage
- [ ] Zero critical bugs in beta
- [ ] GraphQL API available

### v1.0 (Week 20) - 13/15 Required
- [ ] Multi-region HA deployed
- [ ] 99.95% uptime (30 days)
- [ ] AI features functional
- [ ] Compliance integration complete
- [ ] Mobile apps published
- [ ] 500+ incidents/min
- [ ] Load tests passed (2x capacity)
- [ ] Chaos tests passed
- [ ] Security audit passed
- [ ] Documentation complete
- [ ] 50+ customers onboarded
- [ ] 4.5/5 satisfaction
- [ ] 90%+ test coverage
- [ ] 24/7 support established
- [ ] Workflow engine with 10+ templates

---

## Cost Estimates (Monthly Infrastructure)

| Phase | Compute | Database | Cache | LLM API | Services | Observability | Total |
|-------|---------|----------|-------|---------|----------|---------------|-------|
| **MVP** | $500 | $200 | $0 | $300 | $100 | $0 | **$1,100** |
| **Beta** | $1,500 | $600 | $200 | $1,000 | $300 | $200 | **$3,800** |
| **v1.0** | $5,000 | $2,000 | $600 | $3,000 | $800 | $800 | **$13,000** |

**Target Cost per Incident**: < $0.01 (all costs)

---

## API Quick Reference

### Core Endpoints (MVP)

```
POST   /api/v1/incidents           Create incident
GET    /api/v1/incidents           List incidents (paginated)
GET    /api/v1/incidents/:id       Get incident details
PATCH  /api/v1/incidents/:id       Update incident
DELETE /api/v1/incidents/:id       Delete incident

GET    /api/v1/health              Health check
GET    /api/v1/metrics             Prometheus metrics
```

### Authentication
```bash
# API Key (Header)
Authorization: Bearer <api_key>

# JWT (Header)
Authorization: Bearer <jwt_token>
```

### Example Incident Payload
```json
{
  "source": "prometheus",
  "title": "High CPU usage on prod-api-1",
  "description": "CPU usage exceeded 90% for 5 minutes",
  "severity": "HIGH",
  "metadata": {
    "host": "prod-api-1",
    "cpu_percent": 95,
    "timestamp": "2025-11-11T10:00:00Z"
  }
}
```

### Example Response
```json
{
  "id": "inc_abc123",
  "external_id": "prom_alert_456",
  "source": "prometheus",
  "title": "High CPU usage on prod-api-1",
  "severity": "HIGH",
  "priority": "P1",
  "category": "Infrastructure",
  "status": "OPEN",
  "classification": {
    "severity": "HIGH",
    "priority": "P1",
    "category": "Infrastructure",
    "confidence": 0.92
  },
  "created_at": "2025-11-11T10:00:00Z",
  "updated_at": "2025-11-11T10:00:01Z"
}
```

---

## Common Commands

### Development
```bash
# Setup
npm install
cp .env.example .env

# Run locally
npm run dev

# Run tests
npm test
npm run test:integration
npm run test:e2e

# Build
npm run build

# Lint & format
npm run lint
npm run format
```

### Database
```bash
# Run migrations
npm run migrate

# Seed test data
npm run seed

# Backup
pg_dump incident_manager > backup.sql

# Restore
psql incident_manager < backup.sql
```

### Docker
```bash
# Build image
docker build -t llm-incident-manager .

# Run container
docker run -p 3000:3000 llm-incident-manager

# Docker Compose (local dev)
docker-compose up -d
```

### Kubernetes
```bash
# Apply manifests
kubectl apply -f k8s/

# Check deployment
kubectl get pods -n incident-manager

# View logs
kubectl logs -f deployment/incident-manager -n incident-manager

# Port forward
kubectl port-forward svc/incident-manager 3000:3000 -n incident-manager
```

---

## Monitoring & Alerts

### Key Metrics to Monitor
- **Incident ingestion rate** (incidents/min)
- **Classification duration** (seconds, p95/p99)
- **Notification delivery rate** (%, per channel)
- **API response time** (ms, p95/p99)
- **Database query time** (ms, p95/p99)
- **Error rate** (errors/min, by type)
- **LLM API success rate** (%)
- **Cache hit rate** (%)

### Critical Alerts
1. **Service Down**: Health check fails for 2 minutes
2. **High Error Rate**: Error rate > 5% for 5 minutes
3. **Slow Classification**: p95 > 5 seconds for 10 minutes
4. **Failed Notifications**: Delivery rate < 95% for 15 minutes
5. **Database Issues**: Connection pool exhausted or query time > 1s
6. **LLM API Down**: Failure rate > 50% for 5 minutes

### Alert Channels
- **P0/CRITICAL**: PagerDuty (page on-call)
- **P1/HIGH**: Slack + Email
- **P2/MEDIUM**: Slack
- **P3/LOW**: Email (daily digest)

---

## Useful Links

### Documentation
- [Full Implementation Roadmap](./IMPLEMENTATION_ROADMAP.md)
- [Visual Roadmap Summary](./ROADMAP_VISUAL_SUMMARY.md)
- [Technical Decisions (ADRs)](./TECHNICAL_DECISIONS.md)
- [Quick Reference](./QUICK_REFERENCE.md) (this file)

### External Resources
- [Claude API Docs](https://docs.anthropic.com/)
- [Fastify Docs](https://www.fastify.io/)
- [Next.js Docs](https://nextjs.org/docs)
- [PostgreSQL Docs](https://www.postgresql.org/docs/)
- [Kubernetes Docs](https://kubernetes.io/docs/)

---

## Contact & Support

### Team Contacts
- **TPM**: [Contact for roadmap, timeline, resources]
- **Engineering Lead**: [Contact for technical decisions, architecture]
- **DevOps Lead**: [Contact for infrastructure, deployment]
- **Product Manager**: [Contact for features, priorities, customers]

### Meeting Schedule
- **Daily Standup**: 9:00 AM (15 min)
- **Weekly Status**: Mondays 10:00 AM (30 min)
- **Bi-Weekly Demo**: Fridays 2:00 PM (1 hour)
- **Sprint Planning**: Every 2 weeks, Wednesdays 10:00 AM (2 hours)
- **Retrospective**: Every 2 weeks, Fridays 3:00 PM (1 hour)

---

## Next Steps

### Week 0 (Preparation)
1. [ ] Finalize roadmap approval from leadership
2. [ ] Confirm team assignments and availability
3. [ ] Set up development environments and tools
4. [ ] Create GitHub/GitLab repositories
5. [ ] Initial architecture design workshop (1 day)
6. [ ] Kickoff meeting with all stakeholders

### Week 1 (Sprint 1 Start)
1. [ ] Core API design and implementation (ingestion endpoint)
2. [ ] Database schema design and initial migration
3. [ ] LLM integration POC (Claude API)
4. [ ] CI/CD pipeline setup (GitHub Actions)
5. [ ] Project documentation structure
6. [ ] Daily standups and tracking

---

**Document Version**: 1.0
**Last Updated**: 2025-11-11
**Maintained By**: Technical Program Manager
**Review Frequency**: Weekly during active development

**Quick Access**: Bookmark this page for fast reference during development!
