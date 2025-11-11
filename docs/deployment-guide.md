# LLM-Incident-Manager Deployment Guide

## Overview

This guide covers deployment options, configurations, and operational procedures for the LLM-Incident-Manager system.

---

## Table of Contents

1. [Deployment Modes](#deployment-modes)
2. [Prerequisites](#prerequisites)
3. [Standalone Deployment](#standalone-deployment)
4. [Distributed Deployment](#distributed-deployment)
5. [Sidecar Deployment](#sidecar-deployment)
6. [Multi-Region Deployment](#multi-region-deployment)
7. [Configuration](#configuration)
8. [Operations](#operations)
9. [Monitoring](#monitoring)
10. [Troubleshooting](#troubleshooting)

---

## 1. Deployment Modes

### Comparison Matrix

| Feature | Standalone | Distributed | Sidecar | Multi-Region |
|---------|-----------|-------------|---------|--------------|
| Complexity | Low | Medium | Medium | High |
| Scalability | 1K events/min | 100K+ events/min | Per-service | Unlimited |
| HA Support | No | Yes | Yes | Yes |
| Cost | Low | Medium | Low | High |
| Use Case | Dev/Test | Production | Service mesh | Global |

---

## 2. Prerequisites

### System Requirements

**Minimum (Standalone)**:
- CPU: 2 cores
- RAM: 4GB
- Disk: 20GB SSD
- OS: Linux (Ubuntu 20.04+, RHEL 8+, Debian 11+)

**Recommended (Distributed)**:
- CPU: 4+ cores per node
- RAM: 8GB+ per node
- Disk: 50GB+ SSD per node
- Network: 1Gbps+

### Software Dependencies

**Core**:
- Node.js 18+ or Docker 20+
- PostgreSQL 14+ or MongoDB 6+
- Redis 7+

**Optional**:
- Kafka 3+ (for distributed deployment)
- Kubernetes 1.25+ (for container orchestration)
- HAProxy/Nginx (for load balancing)

### Network Requirements

**Inbound**:
- Port 3000: REST API
- Port 9090: gRPC API
- Port 8080: WebSocket
- Port 9092: Metrics (Prometheus)

**Outbound**:
- Database (5432 for PostgreSQL, 27017 for MongoDB)
- Redis (6379)
- Kafka (9092)
- External notification services (SMTP, Slack, PagerDuty, etc.)

---

## 3. Standalone Deployment

### Using Docker Compose

**Step 1: Create directory structure**

```bash
mkdir -p llm-incident-manager/{data,config,logs}
cd llm-incident-manager
```

**Step 2: Create docker-compose.yml**

```yaml
version: '3.8'

services:
  # Main application
  incident-manager:
    image: llm-incident-manager:latest
    container_name: incident-manager
    ports:
      - "3000:3000"   # REST API
      - "9090:9090"   # gRPC API
      - "8080:8080"   # WebSocket
      - "9092:9092"   # Metrics
    environment:
      # Mode
      - MODE=standalone

      # Database
      - DB_TYPE=postgresql
      - DB_HOST=postgres
      - DB_PORT=5432
      - DB_NAME=incident_manager
      - DB_USER=incident_manager
      - DB_PASSWORD=${DB_PASSWORD}

      # Redis
      - REDIS_HOST=redis
      - REDIS_PORT=6379

      # Logging
      - LOG_LEVEL=info
      - LOG_FORMAT=json

      # Features
      - ML_CLASSIFICATION_ENABLED=true
      - AUTO_REMEDIATION_ENABLED=true
    volumes:
      - ./config:/app/config:ro
      - ./data:/app/data
      - ./logs:/app/logs
    depends_on:
      - postgres
      - redis
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  # PostgreSQL database
  postgres:
    image: postgres:15-alpine
    container_name: incident-manager-db
    environment:
      - POSTGRES_DB=incident_manager
      - POSTGRES_USER=incident_manager
      - POSTGRES_PASSWORD=${DB_PASSWORD}
    volumes:
      - postgres-data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U incident_manager"]
      interval: 10s
      timeout: 5s
      retries: 5

  # Redis cache
  redis:
    image: redis:7-alpine
    container_name: incident-manager-redis
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  postgres-data:
  redis-data:

networks:
  default:
    name: incident-manager-network
```

**Step 3: Create configuration file**

```bash
cat > config/config.yaml <<EOF
# LLM-Incident-Manager Configuration

instance_id: "standalone-001"
deployment_mode: standalone

# Database
database:
  type: postgresql
  pool_size: 20
  timeout_ms: 5000

# Cache
cache:
  ttl_seconds: 300
  max_memory_mb: 512

# Notification
notification:
  default_channels:
    - email
    - slack
  rate_limits:
    email:
      requests_per_minute: 60
      burst: 10
    slack:
      requests_per_minute: 100
      burst: 20

# Features
features:
  ml_classification_enabled: true
  auto_remediation_enabled: false
  playbook_execution_enabled: true
  multi_region_enabled: false

# Limits
limits:
  max_events_per_minute: 1000
  max_concurrent_workers: 10
  max_incident_age_days: 90
  max_audit_log_age_days: 365

# Integrations
integrations:
  llm-sentinel:
    enabled: true
    endpoint: https://sentinel.example.com/api
    timeout_ms: 5000
    retry_policy:
      max_attempts: 3
      backoff: exponential
      base_delay_ms: 1000
      max_delay_ms: 30000

  llm-shield:
    enabled: true
    endpoint: https://shield.example.com/api
    timeout_ms: 5000
    retry_policy:
      max_attempts: 3
      backoff: exponential
      base_delay_ms: 1000
      max_delay_ms: 30000
EOF
```

**Step 4: Set environment variables**

```bash
# Create .env file
cat > .env <<EOF
DB_PASSWORD=your_secure_password_here
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/YOUR/WEBHOOK/URL
PAGERDUTY_API_KEY=your_pagerduty_api_key
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USER=your_email@example.com
SMTP_PASSWORD=your_smtp_password
EOF
```

**Step 5: Start services**

```bash
# Start all services
docker-compose up -d

# Check logs
docker-compose logs -f

# Check health
curl http://localhost:3000/health
```

**Step 6: Initialize database**

```bash
# Run migrations
docker-compose exec incident-manager npm run migrate

# Seed initial data (optional)
docker-compose exec incident-manager npm run seed
```

### Using Systemd (Native Installation)

**Step 1: Install dependencies**

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Node.js
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt install -y nodejs

# Install PostgreSQL
sudo apt install -y postgresql postgresql-contrib

# Install Redis
sudo apt install -y redis-server
```

**Step 2: Create database**

```bash
sudo -u postgres psql <<EOF
CREATE DATABASE incident_manager;
CREATE USER incident_manager WITH PASSWORD 'your_password';
GRANT ALL PRIVILEGES ON DATABASE incident_manager TO incident_manager;
EOF
```

**Step 3: Install application**

```bash
# Create user
sudo useradd -r -s /bin/false incident-manager

# Create directories
sudo mkdir -p /opt/incident-manager/{config,data,logs}
sudo chown -R incident-manager:incident-manager /opt/incident-manager

# Install application
cd /opt/incident-manager
sudo -u incident-manager npm install llm-incident-manager

# Copy configuration
sudo cp config/config.yaml.example config/config.yaml
sudo vi config/config.yaml  # Edit configuration
```

**Step 4: Create systemd service**

```bash
sudo cat > /etc/systemd/system/incident-manager.service <<EOF
[Unit]
Description=LLM Incident Manager
After=network.target postgresql.service redis.service
Requires=postgresql.service redis.service

[Service]
Type=simple
User=incident-manager
Group=incident-manager
WorkingDirectory=/opt/incident-manager
ExecStart=/usr/bin/node /opt/incident-manager/dist/index.js
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=incident-manager

# Environment
Environment="NODE_ENV=production"
Environment="CONFIG_PATH=/opt/incident-manager/config/config.yaml"

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/incident-manager/data /opt/incident-manager/logs

[Install]
WantedBy=multi-user.target
EOF
```

**Step 5: Start service**

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service
sudo systemctl enable incident-manager

# Start service
sudo systemctl start incident-manager

# Check status
sudo systemctl status incident-manager

# View logs
sudo journalctl -u incident-manager -f
```

---

## 4. Distributed Deployment

### Kubernetes Deployment

**Architecture**:
- 3+ API servers (horizontal scaling)
- 5+ workers (auto-scaling)
- PostgreSQL cluster (primary + replicas)
- Redis cluster
- Kafka cluster

**Step 1: Create namespace**

```bash
kubectl create namespace incident-manager
```

**Step 2: Create secrets**

```bash
# Database credentials
kubectl create secret generic incident-manager-db \
  --from-literal=username=incident_manager \
  --from-literal=password=your_password \
  -n incident-manager

# API keys
kubectl create secret generic incident-manager-api-keys \
  --from-literal=slack-webhook=https://hooks.slack.com/... \
  --from-literal=pagerduty-key=your_key \
  -n incident-manager
```

**Step 3: Create ConfigMap**

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: incident-manager-config
  namespace: incident-manager
data:
  config.yaml: |
    instance_id: "k8s-cluster-01"
    deployment_mode: distributed

    database:
      type: postgresql
      pool_size: 50
      timeout_ms: 5000

    message_queue:
      type: kafka
      brokers:
        - kafka-broker-1:9092
        - kafka-broker-2:9092
        - kafka-broker-3:9092

    cache:
      type: redis
      ttl_seconds: 300

    limits:
      max_events_per_minute: 100000
      max_concurrent_workers: 100
```

**Step 4: Deploy API servers**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: incident-manager-api
  namespace: incident-manager
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: incident-manager
      component: api
  template:
    metadata:
      labels:
        app: incident-manager
        component: api
    spec:
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
          - labelSelector:
              matchExpressions:
              - key: component
                operator: In
                values:
                - api
            topologyKey: kubernetes.io/hostname
      containers:
      - name: api
        image: llm-incident-manager:latest
        args: ["--mode=api"]
        ports:
        - containerPort: 3000
          name: http
          protocol: TCP
        - containerPort: 9090
          name: grpc
          protocol: TCP
        - containerPort: 9092
          name: metrics
          protocol: TCP
        env:
        - name: DATABASE_URL
          value: postgresql://$(DB_USER):$(DB_PASSWORD)@postgres-primary:5432/incident_manager
        - name: DB_USER
          valueFrom:
            secretKeyRef:
              name: incident-manager-db
              key: username
        - name: DB_PASSWORD
          valueFrom:
            secretKeyRef:
              name: incident-manager-db
              key: password
        - name: REDIS_URL
          value: redis://redis-cluster:6379
        - name: KAFKA_BROKERS
          value: kafka-broker-1:9092,kafka-broker-2:9092,kafka-broker-3:9092
        volumeMounts:
        - name: config
          mountPath: /app/config
          readOnly: true
        resources:
          requests:
            cpu: 500m
            memory: 1Gi
          limits:
            cpu: 2
            memory: 4Gi
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /ready
            port: 3000
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 2
      volumes:
      - name: config
        configMap:
          name: incident-manager-config
---
apiVersion: v1
kind: Service
metadata:
  name: incident-manager-api
  namespace: incident-manager
spec:
  type: ClusterIP
  selector:
    app: incident-manager
    component: api
  ports:
  - name: http
    port: 80
    targetPort: 3000
    protocol: TCP
  - name: grpc
    port: 9090
    targetPort: 9090
    protocol: TCP
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: incident-manager-api
  namespace: incident-manager
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  tls:
  - hosts:
    - incidents.example.com
    secretName: incident-manager-tls
  rules:
  - host: incidents.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: incident-manager-api
            port:
              number: 80
```

**Step 5: Deploy workers**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: incident-manager-classifier
  namespace: incident-manager
spec:
  replicas: 5
  selector:
    matchLabels:
      app: incident-manager
      component: classifier
  template:
    metadata:
      labels:
        app: incident-manager
        component: classifier
    spec:
      containers:
      - name: classifier
        image: llm-incident-manager:latest
        args: ["--mode=worker", "--worker-type=classifier"]
        env:
        - name: DATABASE_URL
          value: postgresql://$(DB_USER):$(DB_PASSWORD)@postgres-primary:5432/incident_manager
        - name: DB_USER
          valueFrom:
            secretKeyRef:
              name: incident-manager-db
              key: username
        - name: DB_PASSWORD
          valueFrom:
            secretKeyRef:
              name: incident-manager-db
              key: password
        - name: KAFKA_CONSUMER_GROUP
          value: classifier-workers
        - name: KAFKA_BROKERS
          value: kafka-broker-1:9092,kafka-broker-2:9092
        volumeMounts:
        - name: config
          mountPath: /app/config
          readOnly: true
        resources:
          requests:
            cpu: 250m
            memory: 512Mi
          limits:
            cpu: 1
            memory: 2Gi
      volumes:
      - name: config
        configMap:
          name: incident-manager-config
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: classifier-worker-hpa
  namespace: incident-manager
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: incident-manager-classifier
  minReplicas: 3
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Pods
    pods:
      metric:
        name: kafka_consumer_lag
      target:
        type: AverageValue
        averageValue: "100"
```

**Step 6: Deploy monitoring**

```yaml
apiVersion: v1
kind: ServiceMonitor
metadata:
  name: incident-manager
  namespace: incident-manager
spec:
  selector:
    matchLabels:
      app: incident-manager
  endpoints:
  - port: metrics
    interval: 30s
    path: /metrics
```

**Step 7: Apply all manifests**

```bash
kubectl apply -f configmap.yaml
kubectl apply -f api-deployment.yaml
kubectl apply -f worker-deployments.yaml
kubectl apply -f monitoring.yaml

# Check deployment status
kubectl get pods -n incident-manager
kubectl get svc -n incident-manager
kubectl get ingress -n incident-manager
```

---

## 5. Sidecar Deployment

### Istio Integration

**Step 1: Enable automatic sidecar injection**

```bash
kubectl label namespace default istio-injection=enabled
```

**Step 2: Deploy application with sidecar**

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: llm-service-with-sidecar
  namespace: default
spec:
  containers:
  # Main application
  - name: llm-service
    image: llm-service:latest
    ports:
    - containerPort: 8000
      name: http
    env:
    - name: INCIDENT_MANAGER_ENDPOINT
      value: "localhost:9091"
    volumeMounts:
    - name: shared-data
      mountPath: /var/shared

  # Incident manager sidecar
  - name: incident-manager-sidecar
    image: llm-incident-manager:sidecar
    args: ["--mode=sidecar", "--service-name=llm-service"]
    ports:
    - containerPort: 9091
      name: grpc
    - containerPort: 9092
      name: metrics
    env:
    - name: CENTRAL_ENDPOINT
      value: "incident-manager-api.incident-manager.svc.cluster.local:9090"
    - name: SERVICE_NAME
      valueFrom:
        fieldRef:
          fieldPath: metadata.name
    - name: BUFFER_SIZE
      value: "1000"
    - name: BATCH_SIZE
      value: "100"
    - name: BATCH_INTERVAL_MS
      value: "5000"
    volumeMounts:
    - name: shared-data
      mountPath: /var/shared
    resources:
      requests:
        cpu: 100m
        memory: 256Mi
      limits:
        cpu: 500m
        memory: 512Mi
    livenessProbe:
      grpc:
        port: 9091
      initialDelaySeconds: 10
      periodSeconds: 10

  volumes:
  - name: shared-data
    emptyDir: {}
```

---

## 6. Multi-Region Deployment

### Global Architecture

**Regions**:
- us-west-2 (Primary)
- eu-west-1 (Secondary)
- ap-south-1 (Secondary)

**Step 1: Deploy CockroachDB (global database)**

```yaml
# Regional clusters with multi-region replication
apiVersion: crdb.cockroachlabs.com/v1alpha1
kind: CrdbCluster
metadata:
  name: incident-manager-db
spec:
  regions:
  - name: us-west-2
    nodeCount: 3
  - name: eu-west-1
    nodeCount: 3
  - name: ap-south-1
    nodeCount: 3
  survivalGoals:
    regionalFailure: true
```

**Step 2: Deploy Kafka (multi-region)**

```yaml
# Kafka MirrorMaker 2 for cross-region replication
apiVersion: kafka.strimzi.io/v1beta2
kind: KafkaMirrorMaker2
metadata:
  name: incident-manager-mm2
spec:
  clusters:
  - alias: us-west-2
    bootstrapServers: kafka-us-west-2:9092
  - alias: eu-west-1
    bootstrapServers: kafka-eu-west-1:9092
  - alias: ap-south-1
    bootstrapServers: kafka-ap-south-1:9092
  mirrors:
  - sourceCluster: us-west-2
    targetCluster: eu-west-1
    sourceConnector: {}
  - sourceCluster: us-west-2
    targetCluster: ap-south-1
    sourceConnector: {}
```

**Step 3: Deploy regional clusters**

Deploy the same Kubernetes manifests to each region, with region-specific configurations.

---

## 7. Configuration

### Configuration File Reference

See `/workspaces/llm-incident-manager/config/config.yaml` for complete configuration options.

### Environment Variables

```bash
# Core
NODE_ENV=production
CONFIG_PATH=/app/config/config.yaml
LOG_LEVEL=info

# Database
DB_TYPE=postgresql
DB_HOST=localhost
DB_PORT=5432
DB_NAME=incident_manager
DB_USER=incident_manager
DB_PASSWORD=secret

# Redis
REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=secret

# Kafka
KAFKA_BROKERS=kafka-1:9092,kafka-2:9092

# Features
ML_CLASSIFICATION_ENABLED=true
AUTO_REMEDIATION_ENABLED=false
```

---

## 8. Operations

### Backup & Restore

**Database Backup**:

```bash
# Backup PostgreSQL
pg_dump -h localhost -U incident_manager incident_manager | gzip > backup-$(date +%Y%m%d).sql.gz

# Restore
gunzip -c backup-20251111.sql.gz | psql -h localhost -U incident_manager incident_manager
```

**Configuration Backup**:

```bash
# Backup config and data
tar -czf backup-config-$(date +%Y%m%d).tar.gz /opt/incident-manager/config /opt/incident-manager/data
```

### Scaling

**Horizontal Scaling (Kubernetes)**:

```bash
# Scale API servers
kubectl scale deployment incident-manager-api --replicas=5 -n incident-manager

# Scale workers
kubectl scale deployment incident-manager-classifier --replicas=10 -n incident-manager
```

**Vertical Scaling**:

```bash
# Update resource limits
kubectl set resources deployment incident-manager-api \
  --limits=cpu=4,memory=8Gi \
  --requests=cpu=2,memory=4Gi \
  -n incident-manager
```

### Upgrades

**Rolling Update**:

```bash
# Update image
kubectl set image deployment/incident-manager-api \
  api=llm-incident-manager:v2.0.0 \
  -n incident-manager

# Monitor rollout
kubectl rollout status deployment/incident-manager-api -n incident-manager

# Rollback if needed
kubectl rollout undo deployment/incident-manager-api -n incident-manager
```

---

## 9. Monitoring

### Prometheus Metrics

See `/workspaces/llm-incident-manager/ARCHITECTURE.md#monitoring--observability` for complete metrics list.

### Grafana Dashboards

Import pre-built dashboards from `/workspaces/llm-incident-manager/monitoring/dashboards/`.

### Alerting

Configure alerts in Prometheus Alertmanager:

```yaml
groups:
- name: incident-manager
  rules:
  - alert: HighIncidentRate
    expr: rate(incidents_created_total[5m]) > 100
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High incident creation rate"
```

---

## 10. Troubleshooting

### Common Issues

**1. Database connection failures**

```bash
# Check database connectivity
psql -h postgres -U incident_manager -d incident_manager -c "SELECT 1"

# Check connection pool
kubectl logs -f deployment/incident-manager-api -n incident-manager | grep "database"
```

**2. High memory usage**

```bash
# Check memory usage
kubectl top pods -n incident-manager

# Restart pod if OOM
kubectl delete pod <pod-name> -n incident-manager
```

**3. Message queue lag**

```bash
# Check Kafka consumer lag
kubectl exec -it kafka-0 -n kafka -- kafka-consumer-groups.sh \
  --bootstrap-server localhost:9092 \
  --describe --group classifier-workers
```

### Debug Mode

```bash
# Enable debug logging
kubectl set env deployment/incident-manager-api LOG_LEVEL=debug -n incident-manager

# Stream logs
kubectl logs -f deployment/incident-manager-api -n incident-manager
```

### Health Checks

```bash
# Check API health
curl http://incidents.example.com/health

# Check readiness
curl http://incidents.example.com/ready

# Check metrics
curl http://incidents.example.com/metrics
```

---

## Appendix

### Port Reference

| Port | Service | Protocol |
|------|---------|----------|
| 3000 | REST API | HTTP |
| 9090 | gRPC API | gRPC |
| 8080 | WebSocket | WS |
| 9092 | Metrics | HTTP |
| 5432 | PostgreSQL | TCP |
| 6379 | Redis | TCP |
| 9092 | Kafka | TCP |

### Resource Limits

| Component | CPU (request) | CPU (limit) | Memory (request) | Memory (limit) |
|-----------|---------------|-------------|------------------|----------------|
| API Server | 500m | 2 | 1Gi | 4Gi |
| Classifier Worker | 250m | 1 | 512Mi | 2Gi |
| Router Worker | 250m | 1 | 512Mi | 2Gi |
| Notifier Worker | 250m | 1 | 512Mi | 2Gi |

### Support

- Documentation: https://docs.example.com/incident-manager
- Issues: https://github.com/globalbusinessadvisors/llm-incident-manager/issues
- Slack: #incident-manager
