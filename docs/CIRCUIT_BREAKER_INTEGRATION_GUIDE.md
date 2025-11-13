# Circuit Breaker Integration Guide

**Version**: 1.0.0
**Last Updated**: 2025-11-13

---

## Table of Contents

1. [Overview](#overview)
2. [Adding Circuit Breakers to New Services](#adding-circuit-breakers-to-new-services)
3. [HTTP Client Integration](#http-client-integration)
4. [gRPC Client Integration](#grpc-client-integration)
5. [Database Integration](#database-integration)
6. [LLM Client Integration](#llm-client-integration)
7. [Custom Integration Examples](#custom-integration-examples)
8. [Migration Guide](#migration-guide)
9. [Testing Your Integration](#testing-your-integration)
10. [Common Patterns](#common-patterns)

---

## Overview

This guide shows you how to integrate circuit breakers into various parts of your system. Circuit breakers should be added to any external dependency that might fail or become slow.

### When to Add Circuit Breakers

Add circuit breakers for:
- External API calls (REST, GraphQL, gRPC)
- Database connections
- Cache servers (Redis, Memcached)
- Message queues
- File system operations (NFS, S3)
- Any network-dependent operation

### Integration Principles

1. **One breaker per dependency**: Each external service gets its own circuit breaker
2. **Fail fast**: Return immediately when circuit is open
3. **Fallback strategy**: Always have a backup plan
4. **Monitor everything**: Track state changes and metrics
5. **Test failures**: Verify circuit breaker behavior under failure

---

## Adding Circuit Breakers to New Services

### Step 1: Identify External Dependencies

List all external dependencies in your service:

```rust
// External dependencies in our service
// 1. Sentinel LLM API (HTTP)
// 2. PostgreSQL Database
// 3. Redis Cache
// 4. Elasticsearch (logging)
```

### Step 2: Create Circuit Breakers

Create one circuit breaker for each dependency:

```rust
use llm_incident_manager::circuit_breaker::CircuitBreaker;
use std::time::Duration;

pub struct ServiceCircuitBreakers {
    pub sentinel: CircuitBreaker,
    pub database: CircuitBreaker,
    pub redis: CircuitBreaker,
    pub elasticsearch: CircuitBreaker,
}

impl ServiceCircuitBreakers {
    pub fn new() -> Self {
        Self {
            sentinel: CircuitBreaker::new("sentinel-api")
                .failure_threshold(5)
                .timeout(Duration::from_secs(60))
                .build(),

            database: CircuitBreaker::new("postgresql")
                .failure_threshold(10)
                .timeout(Duration::from_secs(30))
                .build(),

            redis: CircuitBreaker::new("redis")
                .failure_threshold(3)
                .timeout(Duration::from_secs(10))
                .build(),

            elasticsearch: CircuitBreaker::new("elasticsearch")
                .failure_threshold(5)
                .timeout(Duration::from_secs(60))
                .build(),
        }
    }
}
```

### Step 3: Integrate with Service Struct

Add circuit breakers to your service:

```rust
use std::sync::Arc;

pub struct IncidentService {
    // Existing fields
    db: Arc<DatabasePool>,
    cache: Arc<RedisClient>,
    sentinel_client: Arc<SentinelClient>,

    // Add circuit breakers
    circuit_breakers: Arc<ServiceCircuitBreakers>,
}

impl IncidentService {
    pub fn new(
        db: Arc<DatabasePool>,
        cache: Arc<RedisClient>,
        sentinel_client: Arc<SentinelClient>,
    ) -> Self {
        Self {
            db,
            cache,
            sentinel_client,
            circuit_breakers: Arc::new(ServiceCircuitBreakers::new()),
        }
    }
}
```

### Step 4: Wrap External Calls

Wrap all external calls with circuit breakers:

```rust
impl IncidentService {
    pub async fn fetch_alerts(&self) -> Result<Vec<Alert>, AppError> {
        // Execute through circuit breaker
        self.circuit_breakers.sentinel.call(|| async {
            self.sentinel_client.fetch_alerts(Some(10)).await
                .map_err(|e| AppError::from(e))
        }).await
        .map_err(|e| AppError::from(e))
    }

    pub async fn store_incident(&self, incident: Incident) -> Result<(), AppError> {
        self.circuit_breakers.database.call(|| async {
            self.db.insert_incident(incident).await
                .map_err(|e| AppError::from(e))
        }).await
        .map_err(|e| AppError::from(e))
    }
}
```

---

## HTTP Client Integration

### Basic HTTP Client Integration

```rust
use reqwest::Client;
use llm_incident_manager::circuit_breaker::CircuitBreaker;
use std::sync::Arc;
use std::time::Duration;

pub struct HttpServiceClient {
    client: Client,
    base_url: String,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl HttpServiceClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        let circuit_breaker = Arc::new(
            CircuitBreaker::new("http-service")
                .failure_threshold(5)
                .timeout(Duration::from_secs(60))
                .build()
        );

        Self {
            client,
            base_url,
            circuit_breaker,
        }
    }

    pub async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, AppError> {
        let url = format!("{}/{}", self.base_url, path);

        self.circuit_breaker.call(|| async {
            let response = self.client
                .get(&url)
                .send()
                .await
                .map_err(|e| AppError::Network(e.to_string()))?;

            if !response.status().is_success() {
                return Err(AppError::HttpError {
                    status: response.status().as_u16(),
                    message: response.text().await.unwrap_or_default(),
                });
            }

            response.json::<T>().await
                .map_err(|e| AppError::Serialization(e.to_string()))
        }).await
        .map_err(|e| AppError::from(e))
    }

    pub async fn post<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R, AppError> {
        let url = format!("{}/{}", self.base_url, path);

        self.circuit_breaker.call(|| async {
            let response = self.client
                .post(&url)
                .json(body)
                .send()
                .await
                .map_err(|e| AppError::Network(e.to_string()))?;

            if !response.status().is_success() {
                return Err(AppError::HttpError {
                    status: response.status().as_u16(),
                    message: response.text().await.unwrap_or_default(),
                });
            }

            response.json::<R>().await
                .map_err(|e| AppError::Serialization(e.to_string()))
        }).await
        .map_err(|e| AppError::from(e))
    }
}
```

### Usage Example

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = HttpServiceClient::new("https://api.example.com".to_string());

    // GET request through circuit breaker
    let data: ApiResponse = client.get("data").await?;
    println!("Data: {:?}", data);

    // POST request through circuit breaker
    let request = CreateRequest { name: "test".to_string() };
    let response: CreateResponse = client.post("create", &request).await?;
    println!("Created: {:?}", response);

    Ok(())
}
```

### Advanced: Per-Endpoint Circuit Breakers

```rust
use std::collections::HashMap;

pub struct AdvancedHttpClient {
    client: Client,
    base_url: String,
    circuit_breakers: HashMap<String, Arc<CircuitBreaker>>,
}

impl AdvancedHttpClient {
    pub fn new(base_url: String) -> Self {
        let mut circuit_breakers = HashMap::new();

        // Different configurations for different endpoints
        circuit_breakers.insert(
            "critical".to_string(),
            Arc::new(CircuitBreaker::new("http-critical")
                .failure_threshold(3)
                .timeout(Duration::from_secs(30))
                .build())
        );

        circuit_breakers.insert(
            "standard".to_string(),
            Arc::new(CircuitBreaker::new("http-standard")
                .failure_threshold(5)
                .timeout(Duration::from_secs(60))
                .build())
        );

        Self {
            client: Client::new(),
            base_url,
            circuit_breakers,
        }
    }

    pub async fn get_with_breaker<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        breaker_name: &str,
    ) -> Result<T, AppError> {
        let breaker = self.circuit_breakers
            .get(breaker_name)
            .ok_or_else(|| AppError::Configuration(
                format!("Circuit breaker '{}' not found", breaker_name)
            ))?;

        let url = format!("{}/{}", self.base_url, path);

        breaker.call(|| async {
            self.client.get(&url).send().await?
                .json::<T>().await
        }).await
        .map_err(|e| AppError::from(e))
    }
}
```

---

## gRPC Client Integration

### Basic gRPC Integration

```rust
use tonic::transport::Channel;
use llm_incident_manager::circuit_breaker::CircuitBreaker;
use std::sync::Arc;

// Generated from protobuf
mod proto {
    tonic::include_proto!("incident_service");
}

pub struct GrpcIncidentClient {
    client: proto::incident_service_client::IncidentServiceClient<Channel>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl GrpcIncidentClient {
    pub async fn new(endpoint: String) -> Result<Self, Box<dyn std::error::Error>> {
        let channel = Channel::from_shared(endpoint)?
            .connect()
            .await?;

        let client = proto::incident_service_client::IncidentServiceClient::new(channel);

        let circuit_breaker = Arc::new(
            CircuitBreaker::new("grpc-incident-service")
                .failure_threshold(5)
                .timeout(Duration::from_secs(60))
                .build()
        );

        Ok(Self {
            client,
            circuit_breaker,
        })
    }

    pub async fn create_incident(
        &mut self,
        request: proto::CreateIncidentRequest,
    ) -> Result<proto::Incident, AppError> {
        self.circuit_breaker.call(|| async {
            let response = self.client
                .create_incident(tonic::Request::new(request))
                .await
                .map_err(|e| AppError::Grpc(e.to_string()))?;

            Ok(response.into_inner())
        }).await
        .map_err(|e| AppError::from(e))
    }

    pub async fn get_incident(
        &mut self,
        incident_id: String,
    ) -> Result<proto::Incident, AppError> {
        let request = proto::GetIncidentRequest { incident_id };

        self.circuit_breaker.call(|| async {
            let response = self.client
                .get_incident(tonic::Request::new(request))
                .await
                .map_err(|e| AppError::Grpc(e.to_string()))?;

            Ok(response.into_inner())
        }).await
        .map_err(|e| AppError::from(e))
    }
}
```

### gRPC Streaming with Circuit Breaker

```rust
use tokio_stream::StreamExt;
use tonic::Streaming;

impl GrpcIncidentClient {
    pub async fn stream_incidents(
        &mut self,
        filter: proto::StreamFilter,
    ) -> Result<Streaming<proto::Incident>, AppError> {
        // Circuit breaker for stream initiation
        self.circuit_breaker.call(|| async {
            let response = self.client
                .stream_incidents(tonic::Request::new(filter))
                .await
                .map_err(|e| AppError::Grpc(e.to_string()))?;

            Ok(response.into_inner())
        }).await
        .map_err(|e| AppError::from(e))
    }

    pub async fn consume_stream(
        &mut self,
        filter: proto::StreamFilter,
    ) -> Result<Vec<proto::Incident>, AppError> {
        let mut stream = self.stream_incidents(filter).await?;
        let mut incidents = Vec::new();

        // Each message also goes through circuit breaker
        while let Some(result) = stream.next().await {
            let incident = self.circuit_breaker.call(|| async {
                result.map_err(|e| AppError::Grpc(e.to_string()))
            }).await?;

            incidents.push(incident);
        }

        Ok(incidents)
    }
}
```

---

## Database Integration

### PostgreSQL with Circuit Breaker

```rust
use sqlx::{PgPool, Row};
use llm_incident_manager::circuit_breaker::CircuitBreaker;
use std::sync::Arc;

pub struct DatabaseClient {
    pool: PgPool,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl DatabaseClient {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;

        let circuit_breaker = Arc::new(
            CircuitBreaker::new("postgresql")
                .failure_threshold(10)
                .timeout(Duration::from_secs(30))
                .build()
        );

        Ok(Self {
            pool,
            circuit_breaker,
        })
    }

    pub async fn get_incident(
        &self,
        incident_id: &str,
    ) -> Result<Incident, AppError> {
        self.circuit_breaker.call(|| async {
            let row = sqlx::query(
                "SELECT * FROM incidents WHERE id = $1"
            )
            .bind(incident_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            Ok(Incident::from_row(&row)?)
        }).await
        .map_err(|e| AppError::from(e))
    }

    pub async fn insert_incident(
        &self,
        incident: &Incident,
    ) -> Result<(), AppError> {
        self.circuit_breaker.call(|| async {
            sqlx::query(
                "INSERT INTO incidents (id, title, severity, created_at) \
                 VALUES ($1, $2, $3, $4)"
            )
            .bind(&incident.id)
            .bind(&incident.title)
            .bind(&incident.severity)
            .bind(&incident.created_at)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            Ok(())
        }).await
        .map_err(|e| AppError::from(e))
    }

    pub async fn query_incidents(
        &self,
        filter: &IncidentFilter,
    ) -> Result<Vec<Incident>, AppError> {
        self.circuit_breaker.call(|| async {
            let rows = sqlx::query(
                "SELECT * FROM incidents WHERE severity = $1 AND created_at > $2"
            )
            .bind(&filter.severity)
            .bind(&filter.start_time)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            let incidents = rows.into_iter()
                .map(|row| Incident::from_row(&row))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(incidents)
        }).await
        .map_err(|e| AppError::from(e))
    }
}
```

### Redis Cache with Circuit Breaker

```rust
use redis::{Client, AsyncCommands};
use llm_incident_manager::circuit_breaker::CircuitBreaker;
use std::sync::Arc;

pub struct CacheClient {
    client: Client,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl CacheClient {
    pub fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = Client::open(redis_url)?;

        let circuit_breaker = Arc::new(
            CircuitBreaker::new("redis")
                .failure_threshold(3)
                .timeout(Duration::from_secs(10))
                .build()
        );

        Ok(Self {
            client,
            circuit_breaker,
        })
    }

    pub async fn get<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, AppError> {
        self.circuit_breaker.call(|| async {
            let mut conn = self.client.get_async_connection().await
                .map_err(|e| AppError::Cache(e.to_string()))?;

            let value: Option<String> = conn.get(key).await
                .map_err(|e| AppError::Cache(e.to_string()))?;

            match value {
                Some(v) => {
                    let data = serde_json::from_str(&v)
                        .map_err(|e| AppError::Serialization(e.to_string()))?;
                    Ok(Some(data))
                }
                None => Ok(None),
            }
        }).await
        .map_err(|e| AppError::from(e))
    }

    pub async fn set<T: serde::Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<(), AppError> {
        self.circuit_breaker.call(|| async {
            let mut conn = self.client.get_async_connection().await
                .map_err(|e| AppError::Cache(e.to_string()))?;

            let json = serde_json::to_string(value)
                .map_err(|e| AppError::Serialization(e.to_string()))?;

            conn.set_ex(key, json, ttl.as_secs() as usize).await
                .map_err(|e| AppError::Cache(e.to_string()))?;

            Ok(())
        }).await
        .map_err(|e| AppError::from(e))
    }
}
```

---

## LLM Client Integration

### Sentinel Client with Circuit Breaker

```rust
use llm_incident_manager::integrations::sentinel::SentinelClient;
use llm_incident_manager::circuit_breaker::CircuitBreaker;
use std::sync::Arc;

pub struct ResilientSentinelClient {
    client: SentinelClient,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl ResilientSentinelClient {
    pub fn new(
        config: ConnectionConfig,
        credentials: Credentials,
    ) -> Result<Self, IntegrationError> {
        let client = SentinelClient::new(config, credentials)?;

        let circuit_breaker = Arc::new(
            CircuitBreaker::new("sentinel-llm")
                .failure_threshold(5)
                .timeout(Duration::from_secs(60))
                .success_threshold(2)
                .build()
        );

        Ok(Self {
            client,
            circuit_breaker,
        })
    }

    pub async fn analyze_anomaly_with_fallback(
        &self,
        data: serde_json::Value,
    ) -> Result<AnomalyAnalysis, AppError> {
        match self.circuit_breaker.call(|| async {
            self.client.analyze_anomaly(data.clone()).await
                .map_err(|e| AppError::from(e))
        }).await {
            Ok(analysis) => Ok(analysis),
            Err(e) if e.is_circuit_open() => {
                warn!("Sentinel circuit breaker open, using rule-based fallback");
                Ok(self.rule_based_analysis(data))
            }
            Err(e) => Err(AppError::from(e)),
        }
    }

    fn rule_based_analysis(&self, data: serde_json::Value) -> AnomalyAnalysis {
        // Simple rule-based fallback when LLM is unavailable
        AnomalyAnalysis {
            is_anomalous: false,
            confidence: 0.5,
            anomaly_type: "unknown".to_string(),
            details: Some("Fallback analysis - LLM unavailable".to_string()),
        }
    }
}
```

### Multi-LLM Client with Individual Circuit Breakers

```rust
pub struct MultiLLMClient {
    sentinel: ResilientSentinelClient,
    shield: ResilientShieldClient,
    edge_agent: ResilientEdgeAgentClient,
    governance: ResilientGovernanceClient,
}

impl MultiLLMClient {
    pub fn new(config: &Config) -> Result<Self, AppError> {
        Ok(Self {
            sentinel: ResilientSentinelClient::new(
                config.sentinel.connection.clone(),
                config.sentinel.credentials.clone(),
            )?,
            shield: ResilientShieldClient::new(
                config.shield.connection.clone(),
                config.shield.credentials.clone(),
            )?,
            edge_agent: ResilientEdgeAgentClient::new(
                config.edge_agent.connection.clone(),
                config.edge_agent.credentials.clone(),
            )?,
            governance: ResilientGovernanceClient::new(
                config.governance.connection.clone(),
                config.governance.credentials.clone(),
            )?,
        })
    }

    pub async fn health_check(&self) -> HashMap<String, HealthStatus> {
        let mut health = HashMap::new();

        health.insert(
            "sentinel".to_string(),
            self.sentinel.circuit_breaker.health_check().await
        );
        health.insert(
            "shield".to_string(),
            self.shield.circuit_breaker.health_check().await
        );
        health.insert(
            "edge_agent".to_string(),
            self.edge_agent.circuit_breaker.health_check().await
        );
        health.insert(
            "governance".to_string(),
            self.governance.circuit_breaker.health_check().await
        );

        health
    }
}
```

---

## Custom Integration Examples

### Message Queue Consumer

```rust
use lapin::{Connection, Channel, Consumer};
use llm_incident_manager::circuit_breaker::CircuitBreaker;

pub struct RabbitMQClient {
    connection: Connection,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl RabbitMQClient {
    pub async fn publish(
        &self,
        exchange: &str,
        routing_key: &str,
        payload: Vec<u8>,
    ) -> Result<(), AppError> {
        self.circuit_breaker.call(|| async {
            let channel = self.connection.create_channel().await
                .map_err(|e| AppError::Queue(e.to_string()))?;

            channel.basic_publish(
                exchange,
                routing_key,
                Default::default(),
                &payload,
                Default::default(),
            ).await
            .map_err(|e| AppError::Queue(e.to_string()))?;

            Ok(())
        }).await
        .map_err(|e| AppError::from(e))
    }
}
```

### S3 Client with Circuit Breaker

```rust
use aws_sdk_s3::Client as S3Client;
use llm_incident_manager::circuit_breaker::CircuitBreaker;

pub struct ResilientS3Client {
    client: S3Client,
    bucket: String,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl ResilientS3Client {
    pub async fn get_object(
        &self,
        key: &str,
    ) -> Result<Vec<u8>, AppError> {
        self.circuit_breaker.call(|| async {
            let response = self.client
                .get_object()
                .bucket(&self.bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| AppError::Storage(e.to_string()))?;

            let data = response.body.collect().await
                .map_err(|e| AppError::Storage(e.to_string()))?
                .into_bytes()
                .to_vec();

            Ok(data)
        }).await
        .map_err(|e| AppError::from(e))
    }

    pub async fn put_object(
        &self,
        key: &str,
        data: Vec<u8>,
    ) -> Result<(), AppError> {
        self.circuit_breaker.call(|| async {
            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(key)
                .body(data.into())
                .send()
                .await
                .map_err(|e| AppError::Storage(e.to_string()))?;

            Ok(())
        }).await
        .map_err(|e| AppError::from(e))
    }
}
```

---

## Migration Guide

### Migrating Existing Code to Use Circuit Breakers

#### Before (Without Circuit Breaker)

```rust
pub struct OldServiceClient {
    client: reqwest::Client,
    base_url: String,
}

impl OldServiceClient {
    pub async fn fetch_data(&self) -> Result<Data, AppError> {
        let response = self.client
            .get(&format!("{}/data", self.base_url))
            .send()
            .await?;

        Ok(response.json().await?)
    }
}
```

#### After (With Circuit Breaker)

```rust
pub struct NewServiceClient {
    client: reqwest::Client,
    base_url: String,
    circuit_breaker: Arc<CircuitBreaker>,  // Added
}

impl NewServiceClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            circuit_breaker: Arc::new(  // Added
                CircuitBreaker::new("service-api")
                    .failure_threshold(5)
                    .timeout(Duration::from_secs(60))
                    .build()
            ),
        }
    }

    pub async fn fetch_data(&self) -> Result<Data, AppError> {
        // Wrap the call with circuit breaker
        self.circuit_breaker.call(|| async {
            let response = self.client
                .get(&format!("{}/data", self.base_url))
                .send()
                .await
                .map_err(|e| AppError::from(e))?;

            response.json().await
                .map_err(|e| AppError::from(e))
        }).await
        .map_err(|e| AppError::from(e))
    }
}
```

### Step-by-Step Migration

1. **Add Circuit Breaker Dependency**
   ```toml
   [dependencies]
   llm-incident-manager = { version = "1.0", features = ["circuit-breaker"] }
   ```

2. **Update Struct**
   ```rust
   // Add circuit_breaker field
   pub struct MyClient {
       // existing fields...
       circuit_breaker: Arc<CircuitBreaker>,
   }
   ```

3. **Initialize in Constructor**
   ```rust
   impl MyClient {
       pub fn new(...) -> Self {
           Self {
               // existing initialization...
               circuit_breaker: Arc::new(
                   CircuitBreaker::new("my-service").build()
               ),
           }
       }
   }
   ```

4. **Wrap External Calls**
   ```rust
   // Before
   pub async fn call_service(&self) -> Result<T, E> {
       self.client.call().await
   }

   // After
   pub async fn call_service(&self) -> Result<T, E> {
       self.circuit_breaker.call(|| async {
           self.client.call().await
       }).await
   }
   ```

5. **Add Error Handling**
   ```rust
   match self.circuit_breaker.call(|| async {
       self.client.call().await
   }).await {
       Ok(result) => Ok(result),
       Err(e) if e.is_circuit_open() => {
           // Handle open circuit
           self.fallback()
       }
       Err(e) => Err(e),
   }
   ```

---

## Testing Your Integration

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_opens_on_failures() {
        let breaker = CircuitBreaker::new("test")
            .failure_threshold(3)
            .build();

        // Simulate failures
        for _ in 0..3 {
            let _ = breaker.call(|| async {
                Err::<(), AppError>(AppError::Network("test".to_string()))
            }).await;
        }

        // Circuit should be open
        assert!(breaker.is_open().await);
    }

    #[tokio::test]
    async fn test_circuit_recovery() {
        let breaker = CircuitBreaker::new("test")
            .failure_threshold(2)
            .success_threshold(2)
            .timeout(Duration::from_millis(100))
            .build();

        // Open circuit
        for _ in 0..2 {
            let _ = breaker.call(|| async {
                Err::<(), AppError>(AppError::Network("test".to_string()))
            }).await;
        }

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be half-open
        assert!(breaker.is_half_open().await);

        // Successful requests close circuit
        for _ in 0..2 {
            let _ = breaker.call(|| async {
                Ok::<(), AppError>(())
            }).await;
        }

        assert!(breaker.is_closed().await);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_service_with_circuit_breaker() {
    // Setup mock server
    let mock_server = MockServer::start().await;

    // Create client with circuit breaker
    let client = HttpServiceClient::new(mock_server.uri());

    // Test successful request
    Mock::given(method("GET"))
        .and(path("/data"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
        .mount(&mock_server)
        .await;

    let result: ApiResponse = client.get("data").await.unwrap();
    assert_eq!(result.status, "ok");

    // Test circuit opens on failures
    Mock::given(method("GET"))
        .and(path("/failing"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    for _ in 0..5 {
        let _ = client.get::<ApiResponse>("failing").await;
    }

    // Circuit should be open
    assert!(client.circuit_breaker.is_open().await);

    // Next request should fail fast
    let result = client.get::<ApiResponse>("any-path").await;
    assert!(result.is_err());
}
```

---

## Common Patterns

### Pattern 1: Fallback Chain

```rust
pub async fn fetch_with_fallbacks(&self, key: &str) -> Result<Data, AppError> {
    // Try primary source
    match self.primary_breaker.call(|| self.primary.get(key)).await {
        Ok(data) => return Ok(data),
        Err(e) if e.is_circuit_open() => {
            warn!("Primary circuit open, trying secondary");
        }
        Err(e) => return Err(e.into()),
    }

    // Try secondary source
    match self.secondary_breaker.call(|| self.secondary.get(key)).await {
        Ok(data) => return Ok(data),
        Err(e) if e.is_circuit_open() => {
            warn!("Secondary circuit open, trying cache");
        }
        Err(e) => return Err(e.into()),
    }

    // Try cache
    self.cache_breaker.call(|| self.cache.get(key)).await
        .map_err(|e| e.into())
}
```

### Pattern 2: Parallel Requests with Circuit Breakers

```rust
pub async fn fetch_from_multiple_sources(&self) -> Result<Vec<Data>, AppError> {
    let futures = vec![
        self.source1_breaker.call(|| self.source1.fetch()),
        self.source2_breaker.call(|| self.source2.fetch()),
        self.source3_breaker.call(|| self.source3.fetch()),
    ];

    let results = futures::future::join_all(futures).await;

    // Collect successful results
    let data: Vec<Data> = results.into_iter()
        .filter_map(|r| r.ok())
        .collect();

    if data.is_empty() {
        return Err(AppError::NoDataAvailable);
    }

    Ok(data)
}
```

### Pattern 3: Retry with Circuit Breaker

```rust
pub async fn fetch_with_retry(&self, key: &str) -> Result<Data, AppError> {
    let retry_policy = RetryPolicy::default();

    self.circuit_breaker.call(|| async {
        retry_with_backoff("fetch", &retry_policy, || async {
            self.client.get(key).await
        }).await
    }).await
    .map_err(|e| e.into())
}
```

---

## See Also

- [Circuit Breaker Guide](./CIRCUIT_BREAKER_GUIDE.md) - Complete architectural overview
- [API Reference](./CIRCUIT_BREAKER_API_REFERENCE.md) - Detailed API documentation
- [Operations Guide](./CIRCUIT_BREAKER_OPERATIONS_GUIDE.md) - Monitoring and troubleshooting

---

**Document Version**: 1.0.0
**Last Updated**: 2025-11-13
**Status**: Production Ready
