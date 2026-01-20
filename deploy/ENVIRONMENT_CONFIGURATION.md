# LLM-Incident-Manager Environment Configuration

## 1. Required Environment Variables

All environment variables are resolved from **Google Secret Manager** or **environment configuration**. No agent hardcodes service names, URLs, or credentials.

### 1.1 Core Service Configuration

| Variable | Required | Description | Example |
|----------|----------|-------------|---------|
| `SERVICE_NAME` | Yes | Service identifier | `llm-incident-manager` |
| `SERVICE_VERSION` | Yes | Semantic version | `1.0.1` |
| `PLATFORM_ENV` | Yes | Environment tier | `dev` \| `staging` \| `prod` |
| `GOOGLE_CLOUD_PROJECT` | Yes | GCP project ID | `agentics-dev-platform` |
| `GOOGLE_CLOUD_REGION` | Yes | GCP region | `us-central1` |

### 1.2 RuVector Service (Persistence)

| Variable | Required | Description | Source |
|----------|----------|-------------|--------|
| `RUVECTOR_SERVICE_URL` | Yes | RuVector service endpoint | Environment config |
| `RUVECTOR_API_KEY` | Yes | RuVector authentication | Secret Manager |
| `RUVECTOR_TIMEOUT_MS` | No | Request timeout (default: 30000) | Environment config |

**Schema:**
```
RUVECTOR_SERVICE_URL=https://ruvector-service-{hash}-{region}.a.run.app
```

### 1.3 Telemetry (LLM-Observatory)

| Variable | Required | Description | Source |
|----------|----------|-------------|--------|
| `TELEMETRY_ENDPOINT` | Yes | Observatory ingest endpoint | Environment config |
| `TELEMETRY_API_KEY` | No | Observatory API key (if required) | Secret Manager |
| `TELEMETRY_ENABLED` | No | Enable/disable telemetry (default: true) | Environment config |

**Schema:**
```
TELEMETRY_ENDPOINT=https://llm-observatory-{hash}-{region}.a.run.app/v1/ingest
```

### 1.4 Logging & Tracing

| Variable | Required | Description | Default |
|----------|----------|-------------|---------|
| `RUST_LOG` | No | Log level filter | `info` |
| `LOG_FORMAT` | No | Log format | `json` |
| `OTLP_ENDPOINT` | No | OpenTelemetry collector | (disabled) |
| `OTLP_ENABLED` | No | Enable OTLP export | `false` |

### 1.5 Server Configuration

| Variable | Required | Description | Default |
|----------|----------|-------------|---------|
| `HTTP_PORT` | No | HTTP server port | `8080` |
| `GRPC_PORT` | No | gRPC server port | `9000` |
| `METRICS_PORT` | No | Prometheus port | `9090` |
| `REQUEST_TIMEOUT_SECS` | No | Request timeout | `30` |
| `MAX_CONNECTIONS` | No | Max connections | `10000` |

## 2. Environment-Specific Configuration

### 2.1 Development (`PLATFORM_ENV=dev`)

```bash
# Core
SERVICE_NAME=llm-incident-manager
SERVICE_VERSION=1.0.1-dev
PLATFORM_ENV=dev

# RuVector
RUVECTOR_SERVICE_URL=https://ruvector-service-dev.agentics-dev.run.app
RUVECTOR_API_KEY=sm://agentics-dev-platform/ruvector-api-key-dev

# Telemetry
TELEMETRY_ENDPOINT=https://llm-observatory-dev.agentics-dev.run.app/v1/ingest
TELEMETRY_ENABLED=true

# Logging
RUST_LOG=debug
LOG_FORMAT=text
```

### 2.2 Staging (`PLATFORM_ENV=staging`)

```bash
# Core
SERVICE_NAME=llm-incident-manager
SERVICE_VERSION=1.0.1-rc1
PLATFORM_ENV=staging

# RuVector
RUVECTOR_SERVICE_URL=https://ruvector-service-staging.agentics-staging.run.app
RUVECTOR_API_KEY=sm://agentics-staging/ruvector-api-key

# Telemetry
TELEMETRY_ENDPOINT=https://llm-observatory-staging.agentics-staging.run.app/v1/ingest
TELEMETRY_ENABLED=true

# Logging
RUST_LOG=info
LOG_FORMAT=json
```

### 2.3 Production (`PLATFORM_ENV=prod`)

```bash
# Core
SERVICE_NAME=llm-incident-manager
SERVICE_VERSION=1.0.1
PLATFORM_ENV=prod

# RuVector
RUVECTOR_SERVICE_URL=https://ruvector-service-prod.agentics-prod.run.app
RUVECTOR_API_KEY=sm://agentics-prod/ruvector-api-key

# Telemetry
TELEMETRY_ENDPOINT=https://llm-observatory-prod.agentics-prod.run.app/v1/ingest
TELEMETRY_ENABLED=true

# Logging
RUST_LOG=info,llm_incident_manager=info
LOG_FORMAT=json

# Production hardening
REQUEST_TIMEOUT_SECS=60
MAX_CONNECTIONS=50000
```

## 3. Secret Manager Integration

All secrets are stored in Google Secret Manager and referenced via `sm://` prefix.

### 3.1 Required Secrets

| Secret Name | Description |
|-------------|-------------|
| `ruvector-api-key` | RuVector service authentication |
| `telemetry-api-key` | Observatory API key (if required) |

### 3.2 Secret Resolution

Secrets are resolved at deployment time via Cloud Run secret mounting:

```yaml
# In service.yaml
spec:
  template:
    spec:
      containers:
        - env:
            - name: RUVECTOR_API_KEY
              valueFrom:
                secretKeyRef:
                  name: ruvector-api-key
                  key: latest
```

## 4. Configuration Validation Checklist

Before deployment, verify:

- [ ] No hardcoded service names or URLs in agent code
- [ ] No embedded credentials or secrets in code or config files
- [ ] No inline escalation policies or approval logic
- [ ] All dependencies resolve via environment variables or Secret Manager
- [ ] `RUVECTOR_SERVICE_URL` points to correct environment
- [ ] `TELEMETRY_ENDPOINT` points to correct Observatory instance
- [ ] `PLATFORM_ENV` matches deployment target

## 5. Configuration Loading Chain

Configuration is loaded in the following order (later overrides earlier):

1. **Compiled defaults** in `config/default.toml`
2. **Config file** at `CONFIG_PATH` (default: `/app/config/default.toml`)
3. **Environment variables** with `LLM_IM__` prefix (e.g., `LLM_IM__SERVER__HTTP_PORT=8081`)

### 5.1 Environment Variable Mapping

```
Environment Variable              -> Config Path
LLM_IM__SERVER__HTTP_PORT        -> server.http_port
LLM_IM__RUVECTOR__BASE_URL       -> ruvector.base_url
LLM_IM__TELEMETRY__ENDPOINT      -> telemetry.endpoint
LLM_IM__OBSERVABILITY__LOG_LEVEL -> observability.log_level
```

## 6. Agent-Specific Configuration

### 6.1 Escalation Agent

```bash
# Loaded from packages/agents/escalation-agent/.env
ESCALATION_AGENT_ID=incident-escalation-agent:1.0.0
ESCALATION_AGENT_ENV=${PLATFORM_ENV}
ESCALATION_DEBUG=false
```

### 6.2 HITL Agent

```bash
HITL_AGENT_ID=incident-approver:1.0.0
HITL_AGENT_ENV=${PLATFORM_ENV}
HITL_APPROVAL_TIMEOUT_HOURS=24
```

### 6.3 Post-Mortem Agent

```bash
POSTMORTEM_AGENT_ID=postmortem-generator:1.0.0
POSTMORTEM_AGENT_ENV=${PLATFORM_ENV}
POSTMORTEM_MAX_INCIDENTS=100
```
