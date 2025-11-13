# WebSocket Deployment Guide

## Overview

This guide covers production deployment considerations for the LLM Incident Manager's WebSocket streaming infrastructure. It includes server configuration, scaling strategies, load balancing, monitoring, troubleshooting, and security hardening.

## Table of Contents

- [Server Configuration](#server-configuration)
- [Scaling Considerations](#scaling-considerations)
- [Load Balancing with WebSockets](#load-balancing-with-websockets)
- [Monitoring and Alerting](#monitoring-and-alerting)
- [Troubleshooting Guide](#troubleshooting-guide)
- [Performance Tuning](#performance-tuning)
- [Security Hardening](#security-hardening)

---

## Server Configuration

### Basic Configuration

The WebSocket server is configured through the main application configuration file.

**config.toml:**
```toml
[server]
host = "0.0.0.0"
http_port = 8080              # WebSocket shares HTTP port
grpc_port = 9000
metrics_port = 9090
tls_enabled = true            # Enable for WSS
tls_cert = "/path/to/cert.pem"
tls_key = "/path/to/key.pem"
request_timeout_secs = 30
max_connections = 10000       # Maximum concurrent WebSocket connections

[observability]
prometheus_enabled = true
log_level = "info"
json_logs = true
otlp_enabled = true
otlp_endpoint = "http://otel-collector:4317"
service_name = "llm-incident-manager"

[processing]
max_concurrent_incidents = 10000
correlation_enabled = true
```

### Environment Variables

Override configuration via environment variables:

```bash
export LIM_SERVER_HOST="0.0.0.0"
export LIM_SERVER_HTTP_PORT="8080"
export LIM_SERVER_TLS_ENABLED="true"
export LIM_SERVER_TLS_CERT="/etc/ssl/certs/server.crt"
export LIM_SERVER_TLS_KEY="/etc/ssl/private/server.key"
export LIM_SERVER_MAX_CONNECTIONS="10000"
export LIM_OBSERVABILITY_LOG_LEVEL="info"
```

### TLS/SSL Configuration

**Production WSS Configuration:**

```toml
[server]
tls_enabled = true
tls_cert = "/etc/lim/ssl/fullchain.pem"
tls_key = "/etc/lim/ssl/privkey.pem"
```

**Generate Self-Signed Certificate (Development):**
```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
```

**Let's Encrypt (Production):**
```bash
certbot certonly --standalone -d api.example.com
```

Configure paths:
```toml
tls_cert = "/etc/letsencrypt/live/api.example.com/fullchain.pem"
tls_key = "/etc/letsencrypt/live/api.example.com/privkey.pem"
```

### Systemd Service

**llm-incident-manager.service:**
```ini
[Unit]
Description=LLM Incident Manager
After=network.target

[Service]
Type=simple
User=lim
Group=lim
WorkingDirectory=/opt/llm-incident-manager
Environment="RUST_LOG=info"
Environment="LIM_CONFIG=/etc/lim/config.toml"
ExecStart=/opt/llm-incident-manager/bin/llm-incident-manager
Restart=always
RestartSec=10
LimitNOFILE=65536

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/lim /var/log/lim

[Install]
WantedBy=multi-user.target
```

**Enable and Start:**
```bash
sudo systemctl enable llm-incident-manager
sudo systemctl start llm-incident-manager
sudo systemctl status llm-incident-manager
```

### Docker Deployment

**Dockerfile:**
```dockerfile
FROM rust:1.75 as builder

WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/llm-incident-manager /usr/local/bin/

EXPOSE 8080 9000 9090

ENV RUST_LOG=info
ENV LIM_CONFIG=/etc/lim/config.toml

CMD ["llm-incident-manager"]
```

**docker-compose.yml:**
```yaml
version: '3.8'

services:
  llm-incident-manager:
    build: .
    ports:
      - "8080:8080"   # HTTP/WebSocket
      - "9000:9000"   # gRPC
      - "9090:9090"   # Metrics
    volumes:
      - ./config.toml:/etc/lim/config.toml:ro
      - ./data:/var/lib/lim
      - ./ssl:/etc/lim/ssl:ro
    environment:
      - RUST_LOG=info
      - LIM_SERVER_HOST=0.0.0.0
      - LIM_SERVER_MAX_CONNECTIONS=10000
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

### Kubernetes Deployment

**deployment.yaml:**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-incident-manager
  namespace: monitoring
spec:
  replicas: 3
  selector:
    matchLabels:
      app: llm-incident-manager
  template:
    metadata:
      labels:
        app: llm-incident-manager
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      containers:
      - name: llm-incident-manager
        image: llm-incident-manager:latest
        ports:
        - containerPort: 8080
          name: http
          protocol: TCP
        - containerPort: 9000
          name: grpc
          protocol: TCP
        - containerPort: 9090
          name: metrics
          protocol: TCP
        env:
        - name: RUST_LOG
          value: "info"
        - name: LIM_SERVER_HOST
          value: "0.0.0.0"
        - name: LIM_SERVER_MAX_CONNECTIONS
          value: "10000"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        volumeMounts:
        - name: config
          mountPath: /etc/lim
          readOnly: true
        - name: data
          mountPath: /var/lib/lim
      volumes:
      - name: config
        configMap:
          name: lim-config
      - name: data
        persistentVolumeClaim:
          claimName: lim-data
---
apiVersion: v1
kind: Service
metadata:
  name: llm-incident-manager
  namespace: monitoring
spec:
  type: LoadBalancer
  selector:
    app: llm-incident-manager
  ports:
  - name: http
    port: 8080
    targetPort: 8080
    protocol: TCP
  - name: grpc
    port: 9000
    targetPort: 9000
    protocol: TCP
  - name: metrics
    port: 9090
    targetPort: 9090
    protocol: TCP
  sessionAffinity: ClientIP  # Important for WebSocket
  sessionAffinityConfig:
    clientIP:
      timeoutSeconds: 10800  # 3 hours
```

---

## Scaling Considerations

### Horizontal Scaling

WebSocket connections require special consideration for horizontal scaling.

#### Session Affinity (Sticky Sessions)

**Why It's Needed:**
- WebSocket connections are stateful
- Connection must remain on the same server instance
- Subscription state is maintained in memory

**Implementation:**

**NGINX:**
```nginx
upstream lim_backend {
    ip_hash;  # Session affinity based on client IP
    server lim-1:8080;
    server lim-2:8080;
    server lim-3:8080;
}
```

**Kubernetes:**
```yaml
spec:
  sessionAffinity: ClientIP
  sessionAffinityConfig:
    clientIP:
      timeoutSeconds: 10800
```

#### Event Distribution Across Instances

For multiple server instances to broadcast events to all connected clients, implement a pub/sub mechanism:

**Architecture:**
```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Instance 1 │     │  Instance 2 │     │  Instance 3 │
│  (Clients   │     │  (Clients   │     │  (Clients   │
│   1-100)    │     │   101-200)  │     │   201-300)  │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       └───────────────────┼───────────────────┘
                           │
                    ┌──────▼──────┐
                    │    Redis    │
                    │  Pub/Sub    │
                    └─────────────┘
                           ▲
                           │
                    Incident Update
```

**Implementation (Future Enhancement):**

```rust
// src/streaming/pubsub.rs
use redis::aio::Connection;
use tokio::sync::broadcast;

pub struct EventBroadcaster {
    redis: Connection,
    tx: broadcast::Sender<IncidentUpdate>,
}

impl EventBroadcaster {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let redis = client.get_async_connection().await?;

        let (tx, _) = broadcast::channel(1000);

        Ok(Self { redis, tx })
    }

    pub async fn publish(&mut self, update: &IncidentUpdate) -> Result<()> {
        let payload = serde_json::to_string(update)?;
        redis::cmd("PUBLISH")
            .arg("incidents:updates")
            .arg(payload)
            .query_async(&mut self.redis)
            .await?;
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<IncidentUpdate> {
        self.tx.subscribe()
    }

    pub async fn run_listener(&mut self) -> Result<()> {
        let mut pubsub = self.redis.into_pubsub();
        pubsub.subscribe("incidents:updates").await?;

        while let Some(msg) = pubsub.on_message().next().await {
            if let Ok(payload) = msg.get_payload::<String>() {
                if let Ok(update) = serde_json::from_str(&payload) {
                    let _ = self.tx.send(update);
                }
            }
        }

        Ok(())
    }
}
```

### Vertical Scaling

**Resource Requirements per Connection:**
- Memory: ~50KB per WebSocket connection
- CPU: Minimal (async I/O)

**Capacity Planning:**

| Connections | Memory (Est.) | CPU Cores | Recommended |
|-------------|---------------|-----------|-------------|
| 1,000       | ~50 MB        | 1-2       | t3.small    |
| 10,000      | ~500 MB       | 2-4       | t3.medium   |
| 100,000     | ~5 GB         | 4-8       | t3.xlarge   |
| 1,000,000   | ~50 GB        | 8-16      | Distributed |

**Tokio Worker Threads:**
```rust
#[tokio::main(worker_threads = 8)]
async fn main() {
    // Application code
}
```

Or via environment:
```bash
TOKIO_WORKER_THREADS=8 ./llm-incident-manager
```

---

## Load Balancing with WebSockets

### NGINX Configuration

**nginx.conf:**
```nginx
http {
    upstream lim_backend {
        # Session affinity
        ip_hash;

        server lim-1.internal:8080 max_fails=3 fail_timeout=30s;
        server lim-2.internal:8080 max_fails=3 fail_timeout=30s;
        server lim-3.internal:8080 max_fails=3 fail_timeout=30s;

        # Health checks
        # Requires nginx-plus or use nginx-upstream-check-module
        # check interval=5000 rise=2 fall=3 timeout=1000;
    }

    # WebSocket-aware load balancer
    server {
        listen 80;
        listen 443 ssl http2;
        server_name api.example.com;

        ssl_certificate /etc/ssl/certs/fullchain.pem;
        ssl_certificate_key /etc/ssl/private/privkey.pem;

        # SSL configuration
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers HIGH:!aNULL:!MD5;
        ssl_prefer_server_ciphers on;

        # WebSocket support
        location /graphql/ws {
            proxy_pass http://lim_backend;

            # WebSocket headers
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";

            # Standard proxy headers
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;

            # Timeouts
            proxy_connect_timeout 7d;
            proxy_send_timeout 7d;
            proxy_read_timeout 7d;

            # Buffering
            proxy_buffering off;
        }

        # REST API
        location /graphql {
            proxy_pass http://lim_backend;

            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;

            proxy_connect_timeout 30s;
            proxy_send_timeout 30s;
            proxy_read_timeout 30s;
        }

        # Health check
        location /health {
            proxy_pass http://lim_backend;
            access_log off;
        }

        # Metrics (internal only)
        location /metrics {
            proxy_pass http://lim_backend;
            allow 10.0.0.0/8;
            deny all;
        }
    }
}
```

### HAProxy Configuration

**haproxy.cfg:**
```haproxy
global
    log /dev/log local0
    maxconn 50000
    tune.ssl.default-dh-param 2048

defaults
    log global
    mode http
    option httplog
    timeout connect 10s
    timeout client 7d
    timeout server 7d
    timeout tunnel 7d

frontend http_front
    bind *:80
    bind *:443 ssl crt /etc/ssl/certs/server.pem
    default_backend lim_backend

    # WebSocket detection
    acl is_websocket hdr(Upgrade) -i WebSocket
    acl is_graphql_ws path_beg /graphql/ws

    use_backend lim_ws_backend if is_websocket is_graphql_ws

backend lim_backend
    balance roundrobin
    option httpchk GET /health
    http-check expect status 200

    server lim1 lim-1:8080 check inter 5s fall 3 rise 2
    server lim2 lim-2:8080 check inter 5s fall 3 rise 2
    server lim3 lim-3:8080 check inter 5s fall 3 rise 2

backend lim_ws_backend
    # Sticky sessions for WebSocket
    balance source
    hash-type consistent

    # Health check
    option httpchk GET /health

    # WebSocket servers
    server lim1 lim-1:8080 check inter 5s fall 3 rise 2
    server lim2 lim-2:8080 check inter 5s fall 3 rise 2
    server lim3 lim-3:8080 check inter 5s fall 3 rise 2
```

### AWS Application Load Balancer

**Terraform Configuration:**
```hcl
resource "aws_lb" "lim" {
  name               = "lim-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.lim_alb.id]
  subnets            = var.public_subnet_ids

  enable_deletion_protection = true
  enable_http2              = true

  tags = {
    Name = "lim-alb"
  }
}

resource "aws_lb_target_group" "lim" {
  name     = "lim-tg"
  port     = 8080
  protocol = "HTTP"
  vpc_id   = var.vpc_id

  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 3
    timeout             = 5
    interval            = 30
    path                = "/health"
    matcher             = "200"
  }

  # Sticky sessions for WebSocket
  stickiness {
    type            = "lb_cookie"
    cookie_duration = 86400  # 24 hours
    enabled         = true
  }

  # Connection draining
  deregistration_delay = 300

  tags = {
    Name = "lim-target-group"
  }
}

resource "aws_lb_listener" "https" {
  load_balancer_arn = aws_lb.lim.arn
  port              = "443"
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS-1-2-2017-01"
  certificate_arn   = var.certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.lim.arn
  }
}

# Target group attributes for WebSocket
resource "aws_lb_target_group" "lim" {
  # ... previous config ...

  # Enable WebSocket support
  connection_termination = false

  target_health_state {
    enable_unhealthy_connection_termination = false
  }
}
```

---

## Monitoring and Alerting

### Prometheus Metrics

**Key Metrics to Monitor:**

```promql
# Active WebSocket connections
lim_websocket_connections_active

# Connection rate
rate(lim_websocket_connections_total[5m])

# Subscription count
lim_websocket_subscriptions_active

# Message rate
rate(lim_websocket_messages_sent_total[5m])
rate(lim_websocket_messages_received_total[5m])

# Error rate
rate(lim_websocket_errors_total[5m])

# Connection duration
lim_websocket_connection_duration_seconds
```

**Prometheus Configuration:**
```yaml
scrape_configs:
  - job_name: 'llm-incident-manager'
    static_configs:
      - targets: ['lim-1:9090', 'lim-2:9090', 'lim-3:9090']
    scrape_interval: 15s
    metrics_path: /metrics
```

### Grafana Dashboard

**Example Dashboard JSON:**
```json
{
  "dashboard": {
    "title": "LIM WebSocket Monitoring",
    "panels": [
      {
        "title": "Active Connections",
        "targets": [
          {
            "expr": "sum(lim_websocket_connections_active)"
          }
        ]
      },
      {
        "title": "Connection Rate",
        "targets": [
          {
            "expr": "rate(lim_websocket_connections_total[5m])"
          }
        ]
      },
      {
        "title": "Message Throughput",
        "targets": [
          {
            "expr": "rate(lim_websocket_messages_sent_total[5m])",
            "legendFormat": "Sent"
          },
          {
            "expr": "rate(lim_websocket_messages_received_total[5m])",
            "legendFormat": "Received"
          }
        ]
      },
      {
        "title": "Error Rate",
        "targets": [
          {
            "expr": "rate(lim_websocket_errors_total[5m])"
          }
        ]
      }
    ]
  }
}
```

### Alerting Rules

**prometheus-alerts.yaml:**
```yaml
groups:
  - name: lim_websocket
    interval: 30s
    rules:
      - alert: HighWebSocketErrorRate
        expr: rate(lim_websocket_errors_total[5m]) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High WebSocket error rate detected"
          description: "Error rate is {{ $value | humanize }} errors/sec"

      - alert: WebSocketConnectionsHigh
        expr: lim_websocket_connections_active > 8000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "WebSocket connection count approaching limit"
          description: "Current connections: {{ $value }}"

      - alert: WebSocketConnectionsCritical
        expr: lim_websocket_connections_active > 9500
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "WebSocket connections near capacity"
          description: "Immediate action required. Connections: {{ $value }}"

      - alert: WebSocketMessageBacklog
        expr: lim_websocket_message_queue_size > 1000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "WebSocket message backlog detected"
          description: "Queue size: {{ $value }}"
```

### Log Aggregation

**Fluentd Configuration:**
```yaml
<source>
  @type tail
  path /var/log/lim/application.log
  pos_file /var/log/fluentd/lim.pos
  tag lim.app
  <parse>
    @type json
    time_key timestamp
    time_format %Y-%m-%dT%H:%M:%S.%NZ
  </parse>
</source>

<filter lim.app>
  @type grep
  <regexp>
    key level
    pattern /ERROR|WARN/
  </regexp>
</filter>

<match lim.**>
  @type elasticsearch
  host elasticsearch.internal
  port 9200
  index_name lim-logs
  type_name _doc
  include_tag_key true
  tag_key @log_name
</match>
```

---

## Troubleshooting Guide

### Common Issues

#### 1. Connection Refused

**Symptoms:**
- Client cannot connect
- "Connection refused" error

**Diagnosis:**
```bash
# Check if server is running
systemctl status llm-incident-manager

# Check if port is listening
netstat -tlnp | grep 8080
# or
ss -tlnp | grep 8080

# Test connectivity
curl -i http://localhost:8080/health

# Test WebSocket upgrade
wscat -c ws://localhost:8080/graphql/ws
```

**Solutions:**
- Verify server is running
- Check firewall rules
- Verify port configuration
- Check bind address (0.0.0.0 vs 127.0.0.1)

#### 2. Connection Timeout

**Symptoms:**
- Connection established but times out
- No data received

**Diagnosis:**
```bash
# Check server logs
journalctl -u llm-incident-manager -f

# Monitor active connections
netstat -an | grep ESTABLISHED | grep 8080

# Check load balancer timeout settings
```

**Solutions:**
- Increase timeout values in load balancer
- Implement ping/pong keep-alive
- Check network MTU settings
- Verify proxy configuration

#### 3. Subscription Not Receiving Data

**Symptoms:**
- Connection established
- Subscription created successfully
- No data received

**Diagnosis:**
- Check subscription query syntax
- Verify event publishing is working
- Check filter criteria
- Review server logs for errors

**Solutions:**
- Validate GraphQL query
- Test with minimal filters
- Check correlation engine status
- Verify message broker connectivity (if using Redis Pub/Sub)

#### 4. High Memory Usage

**Symptoms:**
- Memory usage growing over time
- OOM killer terminating process

**Diagnosis:**
```bash
# Monitor memory usage
ps aux | grep llm-incident-manager

# Check process memory map
pmap -x $(pgrep llm-incident-manager)

# Analyze with heaptrack (if available)
heaptrack ./llm-incident-manager
```

**Solutions:**
- Limit maximum connections
- Implement connection idle timeout
- Review subscription cleanup logic
- Check for memory leaks in custom code
- Increase available memory

#### 5. SSL/TLS Errors

**Symptoms:**
- "SSL handshake failed"
- "Certificate verification failed"

**Diagnosis:**
```bash
# Test SSL certificate
openssl s_client -connect api.example.com:443

# Check certificate validity
openssl x509 -in /path/to/cert.pem -text -noout

# Verify certificate chain
openssl verify -CAfile ca.pem cert.pem
```

**Solutions:**
- Renew expired certificates
- Verify certificate chain is complete
- Check certificate permissions
- Update trusted CA certificates
- Verify hostname matches certificate

### Debug Mode

Enable detailed logging:

```toml
[observability]
log_level = "debug"
json_logs = false
```

Or via environment:
```bash
RUST_LOG=debug,llm_incident_manager=trace ./llm-incident-manager
```

### Performance Profiling

**CPU Profiling:**
```bash
# Using perf
perf record -F 99 -p $(pgrep llm-incident-manager) -g -- sleep 60
perf script | stackcollapse-perf.pl | flamegraph.pl > flame.svg

# Using cargo-flamegraph
cargo flamegraph --bin llm-incident-manager
```

**Memory Profiling:**
```bash
# Using valgrind (development builds)
valgrind --leak-check=full ./target/debug/llm-incident-manager

# Using heaptrack
heaptrack ./target/release/llm-incident-manager
heaptrack_gui heaptrack.llm-incident-manager.XXXX.gz
```

---

## Performance Tuning

### Operating System Tuning

**Increase File Descriptor Limits:**
```bash
# /etc/security/limits.conf
lim soft nofile 65536
lim hard nofile 65536

# Verify
ulimit -n
```

**TCP Tuning:**
```bash
# /etc/sysctl.conf
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 8192
net.ipv4.ip_local_port_range = 1024 65535
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 30

# Apply changes
sysctl -p
```

### Application Tuning

**Tokio Configuration:**
```rust
#[tokio::main(worker_threads = 8)]
async fn main() {
    // Tune thread pool size based on workload
}
```

**Connection Limits:**
```toml
[server]
max_connections = 10000
request_timeout_secs = 30
```

**Resource Limits:**
```toml
[processing]
max_concurrent_incidents = 10000

[state]
pool_size = 100  # Database connection pool
```

### Benchmark Results

Expected performance (single instance):
- **Connections:** 10,000+ concurrent
- **Throughput:** 100,000+ messages/sec
- **Latency:** < 10ms (p99)
- **Memory:** ~50KB per connection

---

## Security Hardening

### Network Security

**Firewall Rules:**
```bash
# Allow WebSocket port
ufw allow 8080/tcp comment 'LIM WebSocket'

# Allow only from specific IPs
ufw allow from 10.0.0.0/8 to any port 8080

# Rate limiting
iptables -A INPUT -p tcp --dport 8080 -m limit --limit 25/minute --limit-burst 100 -j ACCEPT
```

**TLS Best Practices:**
```nginx
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers 'ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:...';
ssl_prefer_server_ciphers on;
ssl_session_cache shared:SSL:10m;
ssl_session_timeout 10m;
ssl_stapling on;
ssl_stapling_verify on;
```

### Authentication & Authorization

**JWT Token Validation:**
```rust
// Future enhancement
async fn validate_token(token: &str) -> Result<Claims> {
    let validation = Validation::new(Algorithm::RS256);
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_rsa_pem(PUBLIC_KEY)?,
        &validation
    )?;

    // Check expiration
    if token_data.claims.exp < Utc::now().timestamp() {
        return Err(Error::TokenExpired);
    }

    Ok(token_data.claims)
}
```

**Rate Limiting (Application Level):**
```rust
// Future enhancement
use governor::{Quota, RateLimiter};

let limiter = RateLimiter::direct(
    Quota::per_minute(nonzero!(100u32))
);

// In handler
if limiter.check().is_err() {
    return Err(Error::RateLimitExceeded);
}
```

### Audit Logging

**Security Event Logging:**
```rust
tracing::warn!(
    user = %user_id,
    ip = %client_ip,
    event = "unauthorized_access_attempt",
    subscription = %subscription_id,
    "Unauthorized subscription attempt"
);
```

**Compliance:**
- Log all authentication attempts
- Log subscription creation/termination
- Log access to sensitive data
- Retain logs per compliance requirements

---

## Production Checklist

### Pre-Deployment

- [ ] TLS/SSL certificates configured and valid
- [ ] Environment variables set correctly
- [ ] Configuration file reviewed and validated
- [ ] Resource limits configured
- [ ] Monitoring and alerting set up
- [ ] Log aggregation configured
- [ ] Backup and recovery procedures documented
- [ ] Load testing completed
- [ ] Security scanning performed
- [ ] Disaster recovery plan documented

### Post-Deployment

- [ ] Health checks passing
- [ ] Metrics being collected
- [ ] Logs being aggregated
- [ ] Alerts triggering correctly
- [ ] Load balancer distributing traffic
- [ ] SSL/TLS working correctly
- [ ] Client connections successful
- [ ] Subscriptions receiving data
- [ ] Performance within acceptable ranges
- [ ] Documentation updated

---

## Related Documentation

- [WebSocket Streaming Guide](./WEBSOCKET_STREAMING_GUIDE.md) - Architecture overview
- [WebSocket API Reference](./WEBSOCKET_API_REFERENCE.md) - Complete API documentation
- [WebSocket Client Guide](./WEBSOCKET_CLIENT_GUIDE.md) - Client integration examples
- [Prometheus Metrics Guide](./PROMETHEUS_METRICS_GUIDE.md) - Metrics reference
