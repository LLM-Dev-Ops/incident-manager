# Sprint Planning Template - LLM Incident Manager

## Sprint Overview

**Sprint Number**: [e.g., Sprint 1]
**Phase**: [MVP / Beta / v1.0]
**Duration**: 2 weeks
**Start Date**: [YYYY-MM-DD]
**End Date**: [YYYY-MM-DD]
**Sprint Goal**: [Clear, measurable goal for this sprint]

---

## Team Capacity

| Team Member | Role | Availability | Planned Capacity | Notes |
|-------------|------|--------------|------------------|-------|
| Engineer 1 | Backend | 100% | 10 points | |
| Engineer 2 | Backend | 100% | 10 points | |
| Engineer 3 | Full-Stack | 100% | 10 points | |
| Engineer 4 | DevOps | 50% | 5 points | Shared resource |
| Engineer 5 | QA | 50% | 5 points | Starts Beta phase |

**Total Sprint Capacity**: [Sum of planned capacity] points

---

## Sprint Backlog

### High Priority (Must Have)

#### Story 1: [Title]
**Story Points**: [1-13]
**Assignee**: [Name]
**Priority**: P0/P1

**User Story**:
As a [role], I want to [action], so that [benefit].

**Acceptance Criteria**:
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3

**Technical Notes**:
- Implementation details
- Dependencies
- Risks

**Definition of Done**:
- [ ] Code written and peer reviewed
- [ ] Unit tests written (80%+ coverage)
- [ ] Integration tests written (if applicable)
- [ ] Documentation updated
- [ ] Deployed to dev environment
- [ ] QA validated

---

#### Story 2: [Title]
[Repeat format above]

---

### Medium Priority (Should Have)

#### Story 3: [Title]
[Same format]

---

### Low Priority (Nice to Have)

#### Story 4: [Title]
[Same format]

---

## Technical Debt

| Item | Description | Priority | Effort | Sprint |
|------|-------------|----------|--------|--------|
| TD-1 | [Description] | Medium | 2 points | Sprint 1 |
| TD-2 | [Description] | Low | 3 points | Sprint 2 |

---

## Risks & Blockers

| Risk/Blocker | Impact | Mitigation | Owner | Status |
|--------------|--------|------------|-------|--------|
| [Description] | High/Medium/Low | [Mitigation plan] | [Name] | Open/Resolved |

---

## Dependencies

| Dependency | Type | Owner | Target Date | Status |
|------------|------|-------|-------------|--------|
| [External API access] | External | [Name] | [Date] | Pending |
| [Module integration] | Internal | [Team] | [Date] | In Progress |

---

## Sprint Ceremonies

### Daily Standup
**Time**: 9:00 AM daily
**Duration**: 15 minutes
**Format**: Async (Slack) or Sync (Zoom)

Questions:
1. What did you accomplish yesterday?
2. What will you work on today?
3. Any blockers or concerns?

### Mid-Sprint Check-in
**Time**: Week 1, Friday 3:00 PM
**Duration**: 30 minutes
**Agenda**:
- Progress review (burndown chart)
- Risk assessment
- Adjust priorities if needed

### Sprint Review/Demo
**Time**: Last day of sprint, 2:00 PM
**Duration**: 1 hour
**Attendees**: Team + stakeholders
**Agenda**:
- Demo completed work
- Review sprint goals
- Gather feedback

### Sprint Retrospective
**Time**: Last day of sprint, 3:00 PM
**Duration**: 1 hour
**Attendees**: Team only
**Format**: Start, Stop, Continue

---

## Sprint Metrics

### Velocity Tracking
- **Previous Sprint Velocity**: [points completed]
- **Target Velocity**: [points planned]
- **Actual Velocity**: [points completed] *(filled at end of sprint)*

### Quality Metrics
- **Test Coverage**: [%]
- **Code Review Turnaround**: [hours]
- **Bugs Found**: [count]
- **Bugs Fixed**: [count]

### Performance Metrics
- **Incidents Processed**: [count]
- **API Response Time (p95)**: [ms]
- **Classification Accuracy**: [%]
- **System Uptime**: [%]

---

## Sprint Goals

### Primary Goal
[Clear, measurable goal that aligns with phase objectives]

**Success Criteria**:
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3

### Secondary Goals
1. [Goal 1]
2. [Goal 2]
3. [Goal 3]

---

## Definition of Ready (DoR)

Before a story can be pulled into a sprint:
- [ ] User story clearly defined
- [ ] Acceptance criteria documented
- [ ] Dependencies identified
- [ ] Technical approach discussed
- [ ] Story points estimated
- [ ] No blockers

---

## Definition of Done (DoD)

Before a story can be marked complete:
- [ ] Code written and follows standards
- [ ] Peer reviewed and approved
- [ ] Unit tests written and passing (80%+ coverage)
- [ ] Integration tests written (if applicable)
- [ ] Documentation updated (code comments, README, API docs)
- [ ] Deployed to dev environment
- [ ] QA validated (if QA available)
- [ ] No critical/high bugs
- [ ] Acceptance criteria met

---

## Sample Sprint Plan - Sprint 1 (MVP Weeks 1-2)

### Sprint 1: Core Ingestion & Validation
**Dates**: Week 1-2 of MVP phase
**Goal**: Build functional incident ingestion endpoint with validation

### Team Capacity
- Backend Engineer 1: 10 points
- Backend Engineer 2: 10 points
- Full-Stack Engineer: 10 points
- DevOps Engineer (50%): 5 points
**Total**: 35 points

### Sprint Backlog

#### Story 1: Core Ingestion API
**Points**: 8 | **Assignee**: Backend Engineer 1 | **Priority**: P0

**User Story**:
As an external system, I want to send incidents via HTTP POST, so that they can be processed and managed.

**Acceptance Criteria**:
- [ ] POST /api/v1/incidents endpoint accepts JSON payloads
- [ ] Returns 201 with incident ID on success
- [ ] Returns 400 with error details on invalid input
- [ ] Returns 429 on rate limit exceeded
- [ ] Response time < 200ms (p95)

**Technical Notes**:
- Use Fastify framework
- JSON Schema for validation (Joi/Zod)
- Rate limiting: 100 req/min per source
- UUID for incident IDs

**Tasks**:
- [ ] Set up Fastify server (2 hours)
- [ ] Define incident schema (2 hours)
- [ ] Implement ingestion endpoint (4 hours)
- [ ] Add input validation (3 hours)
- [ ] Implement rate limiting (3 hours)
- [ ] Write unit tests (4 hours)
- [ ] Write API documentation (2 hours)

---

#### Story 2: Database Schema Design
**Points**: 5 | **Assignee**: Backend Engineer 2 | **Priority**: P0

**User Story**:
As a developer, I want a well-designed database schema, so that incidents can be stored reliably and queried efficiently.

**Acceptance Criteria**:
- [ ] PostgreSQL schema defined with all required fields
- [ ] Supports flexible metadata (JSONB)
- [ ] Indexes on frequently queried fields
- [ ] Migration scripts created
- [ ] Can store 1000+ incidents without performance degradation

**Technical Notes**:
- Use node-pg-migrate for migrations
- JSONB for classification and metadata
- Indexes: id, status, severity, created_at

**Tasks**:
- [ ] Design schema (incidents table) (3 hours)
- [ ] Create migration script (2 hours)
- [ ] Add indexes (1 hour)
- [ ] Test with sample data (2 hours)
- [ ] Document schema (2 hours)

---

#### Story 3: In-Memory Queue Setup
**Points**: 5 | **Assignee**: Backend Engineer 1 | **Priority**: P0

**User Story**:
As the system, I want to queue incoming incidents, so that they can be processed asynchronously without blocking ingestion.

**Acceptance Criteria**:
- [ ] In-memory queue implemented
- [ ] Queued incidents processed in order
- [ ] Error handling and retry logic (3 attempts)
- [ ] Queue metrics exposed (depth, processing rate)

**Technical Notes**:
- Use simple array-based queue for MVP
- Prepare for Bull/Redis in Beta

**Tasks**:
- [ ] Implement queue class (3 hours)
- [ ] Add worker processing (3 hours)
- [ ] Implement retry logic (2 hours)
- [ ] Add metrics (2 hours)
- [ ] Write tests (3 hours)

---

#### Story 4: Input Validation & Sanitization
**Points**: 3 | **Assignee**: Backend Engineer 2 | **Priority**: P1

**User Story**:
As the system, I want to validate and sanitize all inputs, so that invalid or malicious data is rejected.

**Acceptance Criteria**:
- [ ] Required fields validated (source, title, description)
- [ ] Optional fields validated if present
- [ ] SQL injection prevention
- [ ] XSS prevention
- [ ] Maximum payload size enforced (1MB)

**Technical Notes**:
- Use Joi or Zod for schema validation
- Sanitize HTML in text fields

**Tasks**:
- [ ] Define validation schemas (2 hours)
- [ ] Implement validation middleware (2 hours)
- [ ] Add sanitization (2 hours)
- [ ] Write validation tests (3 hours)

---

#### Story 5: Development Environment Setup
**Points**: 8 | **Assignee**: DevOps Engineer | **Priority**: P0

**User Story**:
As a developer, I want a reproducible development environment, so that I can start contributing quickly.

**Acceptance Criteria**:
- [ ] Docker Compose setup with all services (API, DB)
- [ ] .env.example with all required variables
- [ ] README with setup instructions
- [ ] Scripts for common tasks (migrate, seed, test)

**Technical Notes**:
- PostgreSQL in Docker
- Node.js app in Docker (hot reload)

**Tasks**:
- [ ] Create Dockerfile (2 hours)
- [ ] Create docker-compose.yml (2 hours)
- [ ] Write setup scripts (3 hours)
- [ ] Document setup process (3 hours)
- [ ] Test on clean machine (2 hours)

---

#### Story 6: CI/CD Pipeline (Basic)
**Points**: 5 | **Assignee**: DevOps Engineer | **Priority**: P1

**User Story**:
As a developer, I want automated testing on every commit, so that I catch bugs early.

**Acceptance Criteria**:
- [ ] CI runs on every PR
- [ ] Runs linting, unit tests, integration tests
- [ ] Reports test coverage
- [ ] Blocks merge if tests fail

**Technical Notes**:
- GitHub Actions or GitLab CI
- Run tests in Docker container

**Tasks**:
- [ ] Create CI workflow file (2 hours)
- [ ] Configure test runners (2 hours)
- [ ] Add coverage reporting (2 hours)
- [ ] Test workflow (2 hours)
- [ ] Document CI process (1 hour)

---

### Sprint 1 Burndown

| Day | Remaining Points | Notes |
|-----|------------------|-------|
| Day 1 | 34 | Story 5 started |
| Day 2 | 32 | |
| Day 3 | 28 | Story 5 completed |
| Day 4 | 24 | Story 1 in progress |
| Day 5 | 20 | |
| Day 6-7 | Weekend | |
| Day 8 | 18 | Story 2 completed |
| Day 9 | 13 | Story 1 completed |
| Day 10 | 8 | Story 3 in progress |

---

### Sprint 1 Review Notes

**Demo**:
- [ ] Show incident ingestion via curl/Postman
- [ ] Show validation error handling
- [ ] Show database with stored incidents
- [ ] Show CI pipeline running

**Feedback**:
[Capture stakeholder feedback]

**Retrospective**:
**Start**:
- [What should we start doing?]

**Stop**:
- [What should we stop doing?]

**Continue**:
- [What should we keep doing?]

**Action Items**:
- [ ] [Action 1] - Owner: [Name]
- [ ] [Action 2] - Owner: [Name]

---

## Sprint Cheat Sheet

### Story Point Reference
| Points | Complexity | Time Estimate | Example |
|--------|-----------|---------------|---------|
| 1 | Trivial | 1-2 hours | Fix typo, update docs |
| 2 | Simple | 2-4 hours | Add validation rule |
| 3 | Easy | 4-8 hours | New API endpoint (simple) |
| 5 | Medium | 1-2 days | Database schema, integration |
| 8 | Complex | 2-4 days | Major feature, complex logic |
| 13 | Very Complex | 4-5 days | Architecture change, multi-system |
| 21 | Epic | Break down! | Should be split into smaller stories |

### Priority Levels
- **P0**: Critical - Blocks release or other work
- **P1**: High - Important for sprint goal
- **P2**: Medium - Nice to have
- **P3**: Low - Backlog

### Sprint Health Indicators

**Healthy Sprint**:
- ✓ Burndown trending toward zero
- ✓ All stories have clear acceptance criteria
- ✓ No blockers lasting > 1 day
- ✓ Test coverage maintained or improved
- ✓ Daily standup participation

**At-Risk Sprint**:
- ⚠ Burndown not decreasing
- ⚠ Unclear requirements or acceptance criteria
- ⚠ Blockers unresolved > 1 day
- ⚠ Test coverage dropping
- ⚠ Missing standups

**Action**: If at-risk, hold mid-sprint adjustment meeting

---

## Communication Templates

### Daily Standup (Async via Slack)
```
**Yesterday**:
- Completed Story 1 (task 1, 2)
- Made progress on Story 2 (50% done)

**Today**:
- Finish Story 2
- Start Story 3 code review

**Blockers**:
- None / [Description of blocker]
```

### Pull Request Template
```
## Description
[Brief description of changes]

## Related Story
Story #[number]: [Title]

## Changes
- Change 1
- Change 2

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manually tested

## Screenshots (if UI change)
[Add screenshots]

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-reviewed code
- [ ] Commented complex logic
- [ ] Documentation updated
- [ ] No new warnings
- [ ] Tests pass locally
```

---

## Useful Commands

```bash
# Start sprint
git checkout -b sprint-1
git push -u origin sprint-1

# Create story branch
git checkout -b story/incident-ingestion

# Run tests before commit
npm test

# Commit with story reference
git commit -m "feat: add incident ingestion endpoint (Story #1)"

# Open PR
gh pr create --title "Story #1: Incident Ingestion" --body "Implements core ingestion API"
```

---

**Document Version**: 1.0
**Last Updated**: 2025-11-11
**Owner**: Technical Program Manager
**Usage**: Copy this template for each sprint, fill in details during planning
