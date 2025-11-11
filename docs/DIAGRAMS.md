# LLM-Incident-Manager System Diagrams

## Overview

This document contains comprehensive ASCII diagrams for the LLM-Incident-Manager architecture.

---

## 1. High-Level System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                              LLM DEVOPS ECOSYSTEM                                        │
│                                                                                          │
│  ┌──────────────┐    ┌─────────────┐    ┌──────────────┐    ┌─────────────────────┐   │
│  │              │    │             │    │              │    │                     │   │
│  │ LLM-Sentinel │    │ LLM-Shield  │    │LLM-Edge-Agent│    │ LLM-Governance-Core │   │
│  │   (Anomaly   │    │ (Security)  │    │  (Runtime    │    │     (Audit &        │   │
│  │   Detection) │    │             │    │   Proxy)     │    │    Compliance)      │   │
│  │              │    │             │    │              │    │                     │   │
│  └──────┬───────┘    └──────┬──────┘    └──────┬───────┘    └──────────┬──────────┘   │
│         │                   │                   │                       │              │
│         │ Events            │ Violations        │ Alerts                │ Queries      │
│         └───────────────────┼───────────────────┼───────────────────────┘              │
│                             │                   │                                      │
│                             ▼                   ▼                                      │
│              ┌──────────────────────────────────────────────────────┐                  │
│              │                                                      │                  │
│              │           LLM-INCIDENT-MANAGER                       │                  │
│              │                                                      │                  │
│              │  ┌────────────────┐  ┌───────────────────────────┐  │                  │
│              │  │   Ingestion    │  │    Classification &       │  │                  │
│              │  │     Layer      │─▶│     Enrichment            │  │                  │
│              │  │  - REST API    │  │  - Severity Classifier    │  │                  │
│              │  │  - gRPC API    │  │  - Deduplication          │  │                  │
│              │  │  - WebSocket   │  │  - Enrichment             │  │                  │
│              │  └────────────────┘  └─────────────┬─────────────┘  │                  │
│              │                                     │                │                  │
│              │                                     ▼                │                  │
│              │  ┌────────────────┐  ┌───────────────────────────┐  │                  │
│              │  │    Routing     │  │      Notification         │  │                  │
│              │  │     Engine     │─▶│        Engine             │  │                  │
│              │  │  - Policy      │  │  - Multi-channel          │  │                  │
│              │  │  - On-call     │  │  - Template rendering     │  │                  │
│              │  │  - Load balance│  │  - Delivery tracking      │  │                  │
│              │  └────────┬───────┘  └───────────────────────────┘  │                  │
│              │           │                                          │                  │
│              │           ▼                                          │                  │
│              │  ┌────────────────┐  ┌───────────────────────────┐  │                  │
│              │  │   Resolution   │  │    Audit & State          │  │                  │
│              │  │     Engine     │─▶│     Management            │  │                  │
│              │  │  - State mgmt  │  │  - Complete audit trail   │  │                  │
│              │  │  - Playbooks   │  │  - Timeline tracking      │  │                  │
│              │  │  - Auto-remedy │  │  - Metrics collection     │  │                  │
│              │  └────────────────┘  └───────────────────────────┘  │                  │
│              │                                                      │                  │
│              └──────────────────┬───────────────┬───────────────────┘                  │
│                                 │               │                                      │
│                                 ▼               ▼                                      │
│         ┌───────────────────────────┐   ┌───────────────────────────┐                 │
│         │    Incident Database      │   │    Message Queue/Bus      │                 │
│         │  (PostgreSQL/MongoDB)     │   │   (Redis/Kafka/RabbitMQ)  │                 │
│         │  - Incidents              │   │   - Event streams         │                 │
│         │  - Policies               │   │   - Worker queues         │                 │
│         │  - Audit logs             │   │   - Dead letter queue     │                 │
│         └───────────────────────────┘   └───────────────────────────┘                 │
│                                                                                         │
└─────────────────────────────────────────────────────────────────────────────────────────┘
                                    │                       │
                                    ▼                       ▼
                ┌────────────────────────────┐   ┌─────────────────────────┐
                │   Notification Channels    │   │   Incident Dashboard    │
                │   - Email (SMTP)           │   │   (Web UI)              │
                │   - Slack/MS Teams         │   │   - View incidents      │
                │   - PagerDuty/OpsGenie     │   │   - Acknowledge/Resolve │
                │   - SMS (Twilio)           │   │   - Analytics           │
                │   - Webhook                │   │   - Configuration       │
                └────────────────────────────┘   └─────────────────────────┘
```

---

## 2. Component Interaction Sequence

```
┌─────────┐  ┌──────────┐  ┌───────────┐  ┌─────────┐  ┌──────────┐  ┌──────────┐
│ Source  │  │Ingestion │  │Classifier │  │ Router  │  │ Notifier │  │Responder │
│ System  │  │  Layer   │  │  Worker   │  │ Worker  │  │  Worker  │  │  (User)  │
└────┬────┘  └────┬─────┘  └─────┬─────┘  └────┬────┘  └────┬─────┘  └────┬─────┘
     │            │               │             │            │             │
     │ 1. POST    │               │             │            │             │
     │   /events  │               │             │            │             │
     ├───────────▶│               │             │            │             │
     │            │               │             │            │             │
     │ 2. 202     │               │             │            │             │
     │   Accepted │               │             │            │             │
     │◀───────────┤               │             │            │             │
     │            │               │             │            │             │
     │            │ 3. Publish    │             │            │             │
     │            │    to Queue   │             │            │             │
     │            ├──────────────▶│             │            │             │
     │            │               │             │            │             │
     │            │               │ 4. Classify │            │             │
     │            │               │    & Enrich │            │             │
     │            │               ├──────────┐  │            │             │
     │            │               │          │  │            │             │
     │            │               │◀─────────┘  │            │             │
     │            │               │             │            │             │
     │            │               │ 5. Create   │            │             │
     │            │               │    Incident │            │             │
     │            │               ├──────────┐  │            │             │
     │            │               │   [DB]   │  │            │             │
     │            │               │◀─────────┘  │            │             │
     │            │               │             │            │             │
     │            │               │ 6. Route    │            │             │
     │            │               ├────────────▶│            │             │
     │            │               │             │            │             │
     │            │               │             │ 7. Notify  │             │
     │            │               │             ├───────────▶│             │
     │            │               │             │            │             │
     │            │               │             │            │ 8. Send     │
     │            │               │             │            │    Alert    │
     │            │               │             │            ├────────────▶│
     │            │               │             │            │             │
     │            │               │             │            │             │ 9. View
     │            │               │             │            │             │    Alert
     │            │               │             │            │             │
     │            │               │             │            │             │ 10. ACK
     │            │               │             │            │             │
     │            │◀──────────────────────────────────────────────────────┤
     │            │               │             │            │             │
     │            │ 11. Update    │             │            │             │
     │            │     State     │             │            │             │
     │            ├──────────┐    │             │            │             │
     │            │   [DB]   │    │             │            │             │
     │            │◀─────────┘    │             │            │             │
     │            │               │             │            │             │
```

---

## 3. Incident Lifecycle State Machine

```
                                ┌─────────────────────────┐
                                │                         │
                                │          NEW            │
                                │    (Initial State)      │
                                │                         │
                                └────────────┬────────────┘
                                             │
                                             │ acknowledge()
                                             ▼
                                ┌─────────────────────────┐
                                │                         │
                                │     ACKNOWLEDGED        │
                                │   (Responder assigned)  │
                                │                         │
                                └────────────┬────────────┘
                                             │
                  ┌──────────────────────────┼──────────────────────────┐
                  │                          │                          │
                  │ escalate()               │ start_work()             │
                  │                          │                          │
                  ▼                          ▼                          │
     ┌─────────────────────────┐  ┌─────────────────────────┐          │
     │                         │  │                         │          │
     │      ESCALATED          │  │     IN_PROGRESS         │          │
     │  (Higher tier involved) │  │   (Active remediation)  │          │
     │                         │  │                         │          │
     └────────────┬────────────┘  └────────────┬────────────┘          │
                  │                            │                       │
                  │ de-escalate()              │ resolve()             │
                  └───────────────────────────▶│                       │
                                               │                       │
                                               ▼                       │
                                  ┌─────────────────────────┐          │
                                  │                         │          │
                                  │       RESOLVED          │          │
                                  │   (Fix applied)         │          │
                                  │                         │          │
                                  └────────────┬────────────┘          │
                                               │                       │
                                               │ close()               │
                                               │                       │
                                               ▼                       │
                                  ┌─────────────────────────┐          │
                                  │                         │          │
                                  │        CLOSED           │          │
                                  │  (Post-mortem done)     │          │
                                  │                         │          │
                                  └─────────────────────────┘          │
                                               │                       │
                                               │ reopen()              │
                                               └───────────────────────┘

Legend:
─────▶ Allowed transitions
action() Required action to transition
```

---

## 4. Data Flow Diagram

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           DATA FLOW ARCHITECTURE                              │
└──────────────────────────────────────────────────────────────────────────────┘

External Sources                     Incident Manager                   Storage
─────────────────                    ────────────────                   ────────

┌──────────────┐
│ LLM-Sentinel │──┐
└──────────────┘  │
                  │
┌──────────────┐  │      ┌────────────────┐
│ LLM-Shield   │──┼─────▶│  API Gateway   │
└──────────────┘  │      │  - Auth        │
                  │      │  - Rate Limit  │
┌──────────────┐  │      │  - Validation  │
│LLM-Edge-Agent│──┘      └────────┬───────┘
└──────────────┘                  │
                                  │ Raw Events
                                  ▼
                         ┌────────────────┐
                         │ Event Buffer   │
                         │  (In-Memory)   │
                         └────────┬───────┘
                                  │
                                  │ Buffered Events
                                  ▼
                         ┌────────────────────┐       ┌──────────────┐
                         │   Message Bus      │──────▶│   Redis      │
                         │  incidents.raw     │       │  (Cache)     │
                         └────────┬───────────┘       └──────────────┘
                                  │                            │
                                  │ Consume                    │ Dedupe
                                  ▼                            │ Check
                         ┌────────────────────┐                │
                         │  Classifier Worker │◀───────────────┘
                         │  - ML Model        │
                         │  - Rule Engine     │
                         └────────┬───────────┘
                                  │
                                  │ Classified Events
                                  ▼
                         ┌────────────────────┐
                         │   Message Bus      │
                         │incidents.classified│
                         └────────┬───────────┘
                                  │
                                  ▼
                         ┌────────────────────┐       ┌──────────────┐
                         │  Incident Creator  │──────▶│ PostgreSQL   │
                         │  - Generate ID     │       │  (Primary)   │
                         │  - Persist         │       │              │
                         └────────┬───────────┘       │ - Incidents  │
                                  │                   │ - Policies   │
                                  │                   │ - Audit Logs │
                                  ▼                   └──────────────┘
                         ┌────────────────────┐
                         │   Message Bus      │
                         │ incidents.routed   │
                         └────────┬───────────┘
                                  │
                                  ▼
                         ┌────────────────────┐
                         │   Router Worker    │
                         │  - Match Policy    │
                         │  - Resolve On-call │
                         └────────┬───────────┘
                                  │
                                  ▼
                         ┌────────────────────┐
                         │   Message Bus      │
                         │incidents.notify    │
                         └────────┬───────────┘
                                  │
                                  ▼
                         ┌────────────────────┐       ┌──────────────┐
                         │ Notifier Worker    │──────▶│   External   │
                         │  - Render Template │       │   Services   │
                         │  - Send via Channel│       │  - Slack     │
                         └────────────────────┘       │  - PagerDuty │
                                                      │  - Email     │
                                                      └──────────────┘

Flow Summary:
1. Events arrive via API Gateway
2. Buffered in-memory for backpressure
3. Published to message bus (incidents.raw)
4. Classifier workers consume and classify
5. Enriched events published to incidents.classified
6. Incident Creator persists to PostgreSQL
7. Routing queue triggers Router workers
8. Notification queue triggers Notifier workers
9. Notifications sent to external channels
```

---

## 5. Deployment Architecture - Kubernetes

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                        KUBERNETES CLUSTER (PRODUCTION)                         │
└───────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                              INGRESS LAYER                                   │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    Ingress Controller (Nginx)                        │   │
│  │  incidents.example.com                                               │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐                              │   │
│  │  │ HTTP/   │  │  gRPC   │  │   WS    │                              │   │
│  │  │ :80/443 │  │  :9090  │  │  :8080  │                              │   │
│  │  └────┬────┘  └────┬────┘  └────┬────┘                              │   │
│  └───────┼────────────┼────────────┼─────────────────────────────────────┘   │
└──────────┼────────────┼────────────┼──────────────────────────────────────┘
           │            │            │
           ▼            ▼            ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           API SERVER TIER                                    │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │              API Server Deployment (ReplicaSet: 3)                   │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               │   │
│  │  │   API Pod 1  │  │   API Pod 2  │  │   API Pod 3  │               │   │
│  │  │  - Ingestion │  │  - Ingestion │  │  - Ingestion │               │   │
│  │  │  - REST API  │  │  - REST API  │  │  - REST API  │               │   │
│  │  │  - gRPC API  │  │  - gRPC API  │  │  - gRPC API  │               │   │
│  │  │  - WebSocket │  │  - WebSocket │  │  - WebSocket │               │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘               │   │
│  │                                                                       │   │
│  │  Service: incident-manager-api (ClusterIP)                          │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           WORKER TIER                                        │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────┐      │
│  │    Classifier Workers (HPA: 3-20 replicas)                       │      │
│  │  [Worker 1] [Worker 2] [Worker 3] ... [Worker N]                 │      │
│  └───────────────────────────────────────────────────────────────────┘      │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────┐      │
│  │    Router Workers (HPA: 2-10 replicas)                           │      │
│  │  [Worker 1] [Worker 2] [Worker 3] ... [Worker N]                 │      │
│  └───────────────────────────────────────────────────────────────────┘      │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────┐      │
│  │    Notifier Workers (HPA: 2-10 replicas)                         │      │
│  │  [Worker 1] [Worker 2] [Worker 3] ... [Worker N]                 │      │
│  └───────────────────────────────────────────────────────────────────┘      │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────┐      │
│  │    Action Workers (HPA: 1-5 replicas)                            │      │
│  │  [Worker 1] [Worker 2] ... [Worker N]                            │      │
│  └───────────────────────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           DATA TIER                                          │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────┐       │
│  │  PostgreSQL StatefulSet (Primary + 2 Replicas)                  │       │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐                       │       │
│  │  │ Primary  │─▶│ Replica1 │  │ Replica2 │                       │       │
│  │  │  (RW)    │  │  (RO)    │  │  (RO)    │                       │       │
│  │  └──────────┘  └──────────┘  └──────────┘                       │       │
│  │                                                                  │       │
│  │  PersistentVolume: 100Gi SSD                                   │       │
│  └──────────────────────────────────────────────────────────────────┘       │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────┐       │
│  │  Redis StatefulSet (Cluster Mode: 6 nodes)                      │       │
│  │  ┌────┐  ┌────┐  ┌────┐  ┌────┐  ┌────┐  ┌────┐                │       │
│  │  │ M1 │──│ M2 │──│ M3 │  │ S1 │  │ S2 │  │ S3 │                │       │
│  │  └────┘  └────┘  └────┘  └────┘  └────┘  └────┘                │       │
│  │  Masters (3)              Slaves (3)                            │       │
│  └──────────────────────────────────────────────────────────────────┘       │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────┐       │
│  │  Kafka StatefulSet (3 brokers + ZooKeeper)                      │       │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐                          │       │
│  │  │Broker 1 │  │Broker 2 │  │Broker 3 │                          │       │
│  │  └─────────┘  └─────────┘  └─────────┘                          │       │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐                 │       │
│  │  │ZooKeeper 1│  │ZooKeeper 2│  │ZooKeeper 3│                 │       │
│  │  └────────────┘  └────────────┘  └────────────┘                 │       │
│  └──────────────────────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                         MONITORING STACK                                     │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │Prometheus  │  │  Grafana   │  │ Alertmngr  │  │   Jaeger   │            │
│  │(Metrics)   │  │(Dashboard) │  │  (Alerts)  │  │  (Tracing) │            │
│  └────────────┘  └────────────┘  └────────────┘  └────────────┘            │
└─────────────────────────────────────────────────────────────────────────────┘

Resource Allocation:
- API Servers: 500m-2 CPU, 1-4Gi RAM each
- Workers: 250m-1 CPU, 512Mi-2Gi RAM each
- PostgreSQL: 2-4 CPU, 8-16Gi RAM, 100Gi storage
- Redis: 1-2 CPU, 4-8Gi RAM
- Kafka: 2-4 CPU, 8-16Gi RAM, 200Gi storage
```

---

## 6. High Availability Architecture

```
┌────────────────────────────────────────────────────────────────────────────┐
│                         MULTI-AZ HA DEPLOYMENT                              │
└────────────────────────────────────────────────────────────────────────────┘

                          ┌─────────────────────┐
                          │   Global Traffic    │
                          │     Manager         │
                          │  (Route53/CF)       │
                          └──────────┬──────────┘
                                     │
           ┌─────────────────────────┼─────────────────────────┐
           │                         │                         │
           ▼                         ▼                         ▼
  ┌────────────────┐       ┌────────────────┐       ┌────────────────┐
  │  AZ-1          │       │  AZ-2          │       │  AZ-3          │
  │  (us-east-1a)  │       │  (us-east-1b)  │       │  (us-east-1c)  │
  │                │       │                │       │                │
  │ ┌────────────┐ │       │ ┌────────────┐ │       │ ┌────────────┐ │
  │ │    ALB     │ │       │ │    ALB     │ │       │ │    ALB     │ │
  │ └──────┬─────┘ │       │ └──────┬─────┘ │       │ └──────┬─────┘ │
  │        │       │       │        │       │       │        │       │
  │ ┌──────▼─────┐ │       │ ┌──────▼─────┐ │       │ ┌──────▼─────┐ │
  │ │ API Servers│ │       │ │ API Servers│ │       │ │ API Servers│ │
  │ │   (2+)     │ │       │ │   (2+)     │ │       │ │   (2+)     │ │
  │ └──────┬─────┘ │       │ └──────┬─────┘ │       │ └──────┬─────┘ │
  │        │       │       │        │       │       │        │       │
  │ ┌──────▼─────┐ │       │ ┌──────▼─────┐ │       │ ┌──────▼─────┐ │
  │ │  Workers   │ │       │ │  Workers   │ │       │ │  Workers   │ │
  │ │  (3+)      │ │       │ │  (3+)      │ │       │ │  (3+)      │ │
  │ └────────────┘ │       │ └────────────┘ │       │ └────────────┘ │
  │                │       │                │       │                │
  └────────┬───────┘       └────────┬───────┘       └────────┬───────┘
           │                        │                        │
           └────────────────────────┼────────────────────────┘
                                    │
                         ┌──────────▼──────────┐
                         │  RDS Multi-AZ      │
                         │  (PostgreSQL)      │
                         │                    │
                         │ ┌────────────────┐ │
                         │ │   Primary      │ │
                         │ │   (AZ-1)       │ │
                         │ └────────┬───────┘ │
                         │          │ Sync    │
                         │          │ Repl    │
                         │ ┌────────▼───────┐ │
                         │ │   Standby      │ │
                         │ │   (AZ-2)       │ │
                         │ └────────────────┘ │
                         │                    │
                         │ Read Replicas:     │
                         │ - AZ-1 (1)         │
                         │ - AZ-2 (1)         │
                         │ - AZ-3 (1)         │
                         └────────────────────┘

                         ┌──────────────────┐
                         │ ElastiCache      │
                         │ (Redis Cluster)  │
                         │                  │
                         │  Shards: 3       │
                         │  Replicas: 2/ea  │
                         │  Multi-AZ: Yes   │
                         └──────────────────┘

Failover Strategy:
1. ALB health checks every 5s
2. Unhealthy instances removed in 15s
3. Auto-scaling replaces failed instances
4. RDS automatic failover in 60-120s
5. Redis Sentinel promotes replica in 30s

SLA Target: 99.95% (21.9 min downtime/month)
RTO: 60 seconds
RPO: 0 seconds (no data loss)
```

---

## 7. Integration Flow Patterns

```
┌────────────────────────────────────────────────────────────────────────────┐
│                    INTEGRATION COMMUNICATION PATTERNS                       │
└────────────────────────────────────────────────────────────────────────────┘

PATTERN 1: PUSH (HTTP/REST)
───────────────────────────

LLM-Sentinel                           Incident-Manager
────────────                           ────────────────

    [Event]
       │
       │ HTTP POST /api/v1/events
       │ X-API-Key: xxx
       │ Content-Type: application/json
       │
       └─────────────────────────────────▶ [Ingestion]
                                                │
                                                │ Validate
                                                │ Authenticate
                                                │ Rate-limit
                                                ▼
                         202 Accepted     [Event Buffer]
       ◀─────────────────────────────────
       {
         "status": "accepted",
         "event_id": "...",
         "incident_id": "..."
       }


PATTERN 2: PUSH (gRPC)
──────────────────────

LLM-Shield                             Incident-Manager
──────────                             ────────────────

    [Violation]
       │
       │ gRPC CreateIncident()
       │ Metadata: authorization: Bearer xxx
       │ mTLS Connection
       │
       └─────────────────────────────────▶ [gRPC Service]
                                                │
                                                │ Validate
                                                │ Verify cert
                                                ▼
                      IncidentResponse    [Create Incident]
       ◀─────────────────────────────────
       {
         incident_id: "...",
         status: "created"
       }


PATTERN 3: STREAMING (WebSocket)
─────────────────────────────────

LLM-Edge-Agent                         Incident-Manager
──────────────                         ────────────────

    [Events]
       │
       │ WebSocket Connect
       │ wss://incidents.example.com/ws
       │ Authorization: Bearer xxx
       │
       └─────────────────────────────────▶ [WS Handler]
                                                │
                  connection_ack           Accept connection
       ◀─────────────────────────────────
       {
         "type": "connection_ack",
         "session_id": "..."
       }
       │
       │ Stream events
       │ { "type": "event", ... }
       │ { "type": "event", ... }
       └─────────────────────────────────▶ [Event Processor]
                                                │
                         ack                    │ Process
       ◀─────────────────────────────────       │
       {                                        ▼
         "type": "ack",                    [Create Incidents]
         "event_id": "..."
       }

       │ Heartbeat (every 30s)
       │ { "type": "heartbeat" }
       └─────────────────────────────────▶


PATTERN 4: PULL (REST)
──────────────────────

LLM-Governance-Core                    Incident-Manager
───────────────────                    ────────────────

    [Report Gen]
       │
       │ HTTP GET /api/v1/incidents
       │ ?start=2025-11-01&end=2025-11-30
       │ Authorization: Bearer xxx
       │
       └─────────────────────────────────▶ [Query API]
                                                │
                                                │ Authenticate
                                                │ Query DB
                                                ▼
                   200 OK                  [Fetch Data]
       ◀─────────────────────────────────
       {
         "incidents": [...],
         "pagination": {...}
       }


PATTERN 5: BIDIRECTIONAL (GraphQL)
───────────────────────────────────

LLM-Governance-Core                    Incident-Manager
───────────────────                    ────────────────

    [Analytics]
       │
       │ POST /graphql
       │ Authorization: Bearer xxx
       │ query {
       │   incidents(filter: {...}) {
       │     edges { node { ... } }
       │   }
       │   metrics { ... }
       │ }
       │
       └─────────────────────────────────▶ [GraphQL API]
                                                │
                                                │ Parse query
                                                │ Resolve data
                                                ▼
                   200 OK                  [Fetch & Agg]
       ◀─────────────────────────────────
       {
         "data": {
           "incidents": {...},
           "metrics": {...}
         }
       }
```

---

## 8. Security Architecture

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        SECURITY LAYERS                                      │
└────────────────────────────────────────────────────────────────────────────┘

                         ┌─────────────────────┐
                         │  External Client    │
                         │  (Sentinel/Shield)  │
                         └──────────┬──────────┘
                                    │
                                    │ 1. TLS 1.3
                                    │    (Encrypted)
                                    ▼
                         ┌─────────────────────┐
                         │  WAF (Web App FW)   │
                         │  - SQL Injection    │
                         │  - XSS Protection   │
                         │  - Rate Limiting    │
                         └──────────┬──────────┘
                                    │
                                    │ 2. DDoS Protection
                                    ▼
                         ┌─────────────────────┐
                         │   API Gateway       │
                         │  - IP Whitelist     │
                         │  - Geo-blocking     │
                         └──────────┬──────────┘
                                    │
                                    │ 3. Authentication
                                    ▼
                         ┌─────────────────────┐
                         │  Auth Middleware    │
                         │  - API Key          │
                         │  - OAuth 2.0        │
                         │  - mTLS             │
                         └──────────┬──────────┘
                                    │
                                    │ 4. Authorization
                                    ▼
                         ┌─────────────────────┐
                         │  RBAC Middleware    │
                         │  - Permissions      │
                         │  - Resource Access  │
                         └──────────┬──────────┘
                                    │
                                    │ 5. Validated Request
                                    ▼
                         ┌─────────────────────┐
                         │  Application Logic  │
                         └──────────┬──────────┘
                                    │
                                    │ 6. Secure Comms
                                    ▼
                         ┌─────────────────────┐
                         │   Database          │
                         │  - Encrypted at rest│
                         │  - TLS in transit   │
                         │  - Access control   │
                         └─────────────────────┘

Security Controls:
─────────────────
1. Network: VPC isolation, Security groups, NACLs
2. Identity: IAM roles, Service accounts, API keys
3. Data: AES-256 encryption, TLS 1.3, Field-level encryption
4. Audit: CloudTrail, Access logs, Audit trail DB
5. Secrets: Vault/Secrets Manager, Key rotation
6. Monitoring: IDS/IPS, Security alerts, SIEM integration
```

This comprehensive diagram collection provides visual representations of all major architectural aspects of the LLM-Incident-Manager system.
