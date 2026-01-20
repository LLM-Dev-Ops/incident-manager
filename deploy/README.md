# LLM-Incident-Manager Deployment Guide

This directory contains all deployment artifacts for the LLM-Incident-Manager service within the Agentics Dev platform.

## Quick Start

```bash
# 1. Set environment
export PROJECT_ID=your-gcp-project
export REGION=us-central1
export ENV=prod  # dev | staging | prod

# 2. Deploy using Cloud Build
gcloud builds submit \
  --config=deploy/cloudbuild.yaml \
  --substitutions=_ENV=${ENV},_REGION=${REGION} \
  --project=${PROJECT_ID}

# 3. Verify deployment
ENV=${ENV} REGION=${REGION} ./deploy/verify-deployment.sh
```

## Deployment Artifacts

| File | Description |
|------|-------------|
| [SERVICE_TOPOLOGY.md](SERVICE_TOPOLOGY.md) | Service architecture and agent endpoints |
| [ENVIRONMENT_CONFIGURATION.md](ENVIRONMENT_CONFIGURATION.md) | Environment variables and secrets |
| [PERSISTENCE_WIRING.md](PERSISTENCE_WIRING.md) | ruvector-service integration, no direct SQL |
| [DEPLOYMENT_COMMANDS.md](DEPLOYMENT_COMMANDS.md) | IAM setup, deployment commands |
| [CLI_ACTIVATION_VERIFICATION.md](CLI_ACTIVATION_VERIFICATION.md) | CLI commands and verification |
| [PLATFORM_INTEGRATION.md](PLATFORM_INTEGRATION.md) | Integration with Sentinel, Orchestrator, etc. |
| [POST_DEPLOY_VERIFICATION.md](POST_DEPLOY_VERIFICATION.md) | Verification checklist |
| [FAILURE_MODES_ROLLBACK.md](FAILURE_MODES_ROLLBACK.md) | Troubleshooting and rollback |
| [cloudbuild.yaml](cloudbuild.yaml) | Cloud Build configuration |
| [service.yaml](service.yaml) | Cloud Run service definition |
| [verify-deployment.sh](verify-deployment.sh) | Automated verification script |

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                  LLM-Incident-Manager                           │
│                  (Unified Cloud Run Service)                    │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │ Escalation  │  │    HITL     │  │    Post-Mortem          │ │
│  │   Agent     │  │    Agent    │  │    Generator            │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                    Shared Services                              │
│  RuvectorClient │ TelemetryEmitter │ DecisionEventBuilder      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │ ruvector-service│
                    │ (Google SQL)    │
                    └─────────────────┘
```

## Key Architectural Constraints

1. **Unified Service**: All agents deploy as endpoints within ONE Cloud Run service
2. **No Direct SQL**: All persistence via ruvector-service
3. **Stateless**: Service is completely stateless
4. **Internal Only**: Service only accepts internal/load-balanced traffic
5. **DecisionEvent**: Every agent invocation emits exactly ONE DecisionEvent

## Prerequisites

- GCP Project with required APIs enabled
- Service account with minimal permissions
- VPC connector for internal communication
- Secrets in Secret Manager

See [DEPLOYMENT_COMMANDS.md](DEPLOYMENT_COMMANDS.md) for detailed setup.

## Deployment Environments

| Environment | Min Instances | Max Instances | Memory |
|-------------|---------------|---------------|--------|
| dev | 1 | 5 | 2Gi |
| staging | 2 | 8 | 2Gi |
| prod | 3 | 10 | 4Gi |

## Rollback

```bash
# List revisions
gcloud run revisions list --service=llm-incident-manager --region=${REGION}

# Rollback to previous revision
gcloud run services update-traffic llm-incident-manager \
  --region=${REGION} \
  --to-revisions=llm-incident-manager-00002-xyz=100
```

See [FAILURE_MODES_ROLLBACK.md](FAILURE_MODES_ROLLBACK.md) for detailed procedures.

## Support

- Platform Team: platform-team@example.com
- On-call: See PagerDuty escalation policy
