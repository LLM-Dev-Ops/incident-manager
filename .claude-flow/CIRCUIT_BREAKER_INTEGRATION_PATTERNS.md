# Circuit Breaker Integration Patterns

## Overview

This document defines comprehensive integration patterns for applying circuit breakers to different service types in the LLM Incident Manager, including HTTP clients, gRPC services, database connections, and generic async operations.

## Integration Strategies

### 1. Middleware Pattern (HTTP Clients)

Apply circuit breakers transparently using middleware for HTTP clients.

#### Implementation

```rust
use tower::{Service, ServiceBuilder, ServiceExt};
use std::task::{Context, Poll};

/// HTTP Circuit Breaker Middleware
pub struct CircuitBreakerMiddleware<S> {
    inner: S,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl<S> CircuitBreakerMiddleware<S> {
    pub fn new(inner: S, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            inner,
            circuit_breaker,
        }
    }
}

impl<S, Request> Service<Request> for CircuitBreakerMiddleware<S>
where
    S: Service<Request>,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = S::Response;
    type Error = CircuitBreakerError<S::Error>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner
            .poll_ready(cx)
            .map_err(CircuitBreakerError::OperationFailed)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let circuit_breaker = self.circuit_breaker.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            circuit_breaker
                .call(|| async move {
                    inner
                        .call(req)
                        .await
                        .map_err(|e| e.into())
                })
                .await
        })
    }
}

/// Circuit Breaker Layer for Tower
pub struct CircuitBreakerLayer {
    circuit_breaker: Arc<CircuitBreaker>,
}

impl CircuitBreakerLayer {
    pub fn new(circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self { circuit_breaker }
    }
}

impl<S> tower::Layer<S> for CircuitBreakerLayer {
    type Service = CircuitBreakerMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CircuitBreakerMiddleware::new(inner, self.circuit_breaker.clone())
    }
}
```

#### Usage with reqwest

```rust
use reqwest::Client;
use tower::ServiceBuilder;

pub struct ResilientHttpClient {
    client: Client,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl ResilientHttpClient {
    pub fn new(name: &str, config: CircuitBreakerConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        let circuit_breaker = Arc::new(CircuitBreaker::new(name.to_string(), config));

        Self {
            client,
            circuit_breaker,
        }
    }

    pub async fn get(&self, url: &str) -> Result<reqwest::Response, CircuitBreakerError<reqwest::Error>> {
        self.circuit_breaker
            .call(|| async {
                self.client
                    .get(url)
                    .send()
                    .await
            })
            .await
    }

    pub async fn post<T: Serialize>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<reqwest::Response, CircuitBreakerError<reqwest::Error>> {
        self.circuit_breaker
            .call(|| async {
                self.client
                    .post(url)
                    .json(body)
                    .send()
                    .await
            })
            .await
    }
}
```

#### Integration with Existing LLM Clients

```rust
use crate::integrations::sentinel::SentinelClient;

impl SentinelClient {
    /// Wrap with circuit breaker
    pub fn with_circuit_breaker(mut self, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        self.circuit_breaker = Some(circuit_breaker);
        self
    }

    /// Fetch alerts with circuit breaker protection
    pub async fn fetch_alerts_protected(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<SentinelAlert>, CircuitBreakerError<IntegrationError>> {
        if let Some(ref cb) = self.circuit_breaker {
            cb.call(|| self.fetch_alerts(limit)).await
        } else {
            self.fetch_alerts(limit)
                .await
                .map_err(CircuitBreakerError::OperationFailed)
        }
    }
}
```

### 2. Decorator Pattern (Function Wrapping)

Wrap individual functions or methods with circuit breaker protection.

#### Implementation

```rust
/// Decorator for async functions
pub struct CircuitBreakerDecorator<F, Fut, T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error,
{
    func: F,
    circuit_breaker: Arc<CircuitBreaker>,
    _phantom: PhantomData<(T, E)>,
}

impl<F, Fut, T, E> CircuitBreakerDecorator<F, Fut, T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error,
{
    pub fn new(func: F, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            func,
            circuit_breaker,
            _phantom: PhantomData,
        }
    }

    pub async fn call(&self) -> Result<T, CircuitBreakerError<E>> {
        self.circuit_breaker.call(&self.func).await
    }
}

/// Macro for easy decoration
#[macro_export]
macro_rules! with_circuit_breaker {
    ($cb:expr, $func:expr) => {
        $cb.call(|| async { $func.await })
    };
}
```

#### Usage

```rust
// Decorate a function
async fn risky_operation() -> Result<String, MyError> {
    // ... operation that might fail
}

let cb = Arc::new(CircuitBreaker::new("risky_op".to_string(), config));
let decorated = CircuitBreakerDecorator::new(risky_operation, cb);

// Call the decorated function
let result = decorated.call().await?;

// Or use the macro
let result = with_circuit_breaker!(cb, risky_operation()).await?;
```

### 3. Interceptor Pattern (gRPC)

Apply circuit breakers to gRPC clients using interceptors.

#### Implementation

```rust
use tonic::{Request, Response, Status};
use tonic::service::Interceptor;

/// gRPC Circuit Breaker Interceptor
pub struct GrpcCircuitBreakerInterceptor {
    circuit_breaker: Arc<CircuitBreaker>,
}

impl GrpcCircuitBreakerInterceptor {
    pub fn new(circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self { circuit_breaker }
    }
}

impl Interceptor for GrpcCircuitBreakerInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        // Check circuit state before allowing request
        if self.circuit_breaker.is_open() {
            return Err(Status::unavailable("Circuit breaker is open"));
        }

        Ok(request)
    }
}

/// gRPC Client with Circuit Breaker
pub struct ResilientGrpcClient<T> {
    client: T,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl<T> ResilientGrpcClient<T>
where
    T: Clone,
{
    pub fn new(client: T, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            client,
            circuit_breaker,
        }
    }

    /// Execute gRPC call with circuit breaker
    pub async fn call<F, Fut, R>(&self, operation: F) -> Result<Response<R>, CircuitBreakerError<Status>>
    where
        F: FnOnce(T) -> Fut,
        Fut: Future<Output = Result<Response<R>, Status>>,
    {
        self.circuit_breaker
            .call(|| async {
                let client = self.client.clone();
                operation(client).await
            })
            .await
    }
}
```

#### Usage with tonic

```rust
use tonic::transport::Channel;
use proto::incident_service_client::IncidentServiceClient;

// Create gRPC client with circuit breaker
let channel = Channel::from_static("http://localhost:50051")
    .connect()
    .await?;

let grpc_client = IncidentServiceClient::new(channel);

let cb = Arc::new(CircuitBreaker::new(
    "incident_service".to_string(),
    CircuitBreakerConfig::default(),
));

let resilient_client = ResilientGrpcClient::new(grpc_client, cb);

// Make protected gRPC call
let response = resilient_client
    .call(|mut client| async move {
        client.get_incident(Request::new(GetIncidentRequest {
            incident_id: "123".to_string(),
        }))
        .await
    })
    .await?;
```

### 4. Trait-Based Abstraction

Define a trait for circuit breaker-protected operations.

#### Implementation

```rust
use async_trait::async_trait;

/// Trait for circuit breaker protected operations
#[async_trait]
pub trait CircuitProtected {
    type Output;
    type Error: std::error::Error;

    /// Execute with circuit breaker protection
    async fn execute_protected(&self) -> Result<Self::Output, CircuitBreakerError<Self::Error>>;

    /// Get the circuit breaker
    fn circuit_breaker(&self) -> &Arc<CircuitBreaker>;

    /// Get circuit breaker state
    fn circuit_state(&self) -> CircuitState {
        self.circuit_breaker().state()
    }

    /// Check if circuit is healthy
    fn is_healthy(&self) -> bool {
        self.circuit_breaker().is_closed()
    }
}

/// Generic implementation for any client
pub struct ProtectedClient<C, F, Fut, T, E>
where
    F: Fn(&C) -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error,
{
    client: C,
    operation: F,
    circuit_breaker: Arc<CircuitBreaker>,
    _phantom: PhantomData<(T, E)>,
}

#[async_trait]
impl<C, F, Fut, T, E> CircuitProtected for ProtectedClient<C, F, Fut, T, E>
where
    C: Send + Sync,
    F: Fn(&C) -> Fut + Send + Sync,
    Fut: Future<Output = Result<T, E>> + Send,
    T: Send,
    E: std::error::Error + Send,
{
    type Output = T;
    type Error = E;

    async fn execute_protected(&self) -> Result<Self::Output, CircuitBreakerError<Self::Error>> {
        self.circuit_breaker
            .call(|| (self.operation)(&self.client))
            .await
    }

    fn circuit_breaker(&self) -> &Arc<CircuitBreaker> {
        &self.circuit_breaker
    }
}
```

#### Usage

```rust
// Implement for Sentinel client
#[async_trait]
impl CircuitProtected for SentinelClient {
    type Output = Vec<SentinelAlert>;
    type Error = IntegrationError;

    async fn execute_protected(&self) -> Result<Self::Output, CircuitBreakerError<Self::Error>> {
        self.circuit_breaker()
            .call(|| self.fetch_alerts(None))
            .await
    }

    fn circuit_breaker(&self) -> &Arc<CircuitBreaker> {
        &self.circuit_breaker
    }
}

// Use the trait
let client = SentinelClient::new(config, credentials)?;
let alerts = client.execute_protected().await?;
```

### 5. Connection Pool Integration

Apply circuit breakers to database connection pools.

#### Implementation

```rust
use sqlx::{Pool, Postgres, Error as SqlxError};

/// Database connection pool with circuit breaker
pub struct ResilientDbPool {
    pool: Pool<Postgres>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl ResilientDbPool {
    pub fn new(pool: Pool<Postgres>, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            pool,
            circuit_breaker,
        }
    }

    /// Execute query with circuit breaker protection
    pub async fn execute<F, Fut, T>(
        &self,
        operation: F,
    ) -> Result<T, CircuitBreakerError<SqlxError>>
    where
        F: FnOnce(Pool<Postgres>) -> Fut,
        Fut: Future<Output = Result<T, SqlxError>>,
    {
        self.circuit_breaker
            .call(|| {
                let pool = self.pool.clone();
                operation(pool)
            })
            .await
    }

    /// Get connection with circuit breaker check
    pub async fn get_connection(
        &self,
    ) -> Result<sqlx::pool::PoolConnection<Postgres>, CircuitBreakerError<SqlxError>> {
        self.circuit_breaker
            .call(|| async { self.pool.acquire().await.map_err(|e| e.into()) })
            .await
    }
}
```

#### Usage

```rust
// Create pool with circuit breaker
let pool = PgPoolOptions::new()
    .max_connections(20)
    .connect(&database_url)
    .await?;

let cb = Arc::new(CircuitBreaker::new(
    "postgres_pool".to_string(),
    CircuitBreakerConfig::default(),
));

let resilient_pool = ResilientDbPool::new(pool, cb);

// Execute query with protection
let incidents = resilient_pool
    .execute(|pool| async move {
        sqlx::query_as::<_, Incident>("SELECT * FROM incidents WHERE status = 'new'")
            .fetch_all(&pool)
            .await
    })
    .await?;
```

### 6. Async Stream Protection

Protect async streams with circuit breakers.

#### Implementation

```rust
use futures::Stream;
use pin_project::pin_project;

/// Circuit breaker protected stream
#[pin_project]
pub struct CircuitBreakerStream<S, T, E>
where
    S: Stream<Item = Result<T, E>>,
    E: std::error::Error,
{
    #[pin]
    stream: S,
    circuit_breaker: Arc<CircuitBreaker>,
    _phantom: PhantomData<(T, E)>,
}

impl<S, T, E> CircuitBreakerStream<S, T, E>
where
    S: Stream<Item = Result<T, E>>,
    E: std::error::Error,
{
    pub fn new(stream: S, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            stream,
            circuit_breaker,
            _phantom: PhantomData,
        }
    }
}

impl<S, T, E> Stream for CircuitBreakerStream<S, T, E>
where
    S: Stream<Item = Result<T, E>>,
    E: std::error::Error,
{
    type Item = Result<T, CircuitBreakerError<E>>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();

        // Check circuit state
        if this.circuit_breaker.is_open() {
            return Poll::Ready(Some(Err(CircuitBreakerError::CircuitOpen)));
        }

        // Poll the underlying stream
        match this.stream.poll_next(cx) {
            Poll::Ready(Some(Ok(item))) => {
                // Record success
                // Note: This is simplified; full implementation would track properly
                Poll::Ready(Some(Ok(item)))
            }
            Poll::Ready(Some(Err(e))) => {
                // Record failure
                Poll::Ready(Some(Err(CircuitBreakerError::OperationFailed(e))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
```

#### Usage with gRPC streaming

```rust
// Protect a gRPC streaming response
let stream = client
    .stream_incidents(Request::new(StreamIncidentsRequest {}))
    .await?
    .into_inner();

let cb = Arc::new(CircuitBreaker::new(
    "incident_stream".to_string(),
    config,
));

let protected_stream = CircuitBreakerStream::new(stream, cb);

// Consume the protected stream
while let Some(result) = protected_stream.next().await {
    match result {
        Ok(incident) => println!("Received incident: {:?}", incident),
        Err(CircuitBreakerError::CircuitOpen) => {
            eprintln!("Circuit is open, using fallback");
            break;
        }
        Err(e) => eprintln!("Error: {:?}", e),
    }
}
```

## Service-Specific Integration Examples

### Sentinel Client Integration

```rust
// src/integrations/sentinel/client.rs
impl SentinelClient {
    pub fn new_with_circuit_breaker(
        config: ConnectionConfig,
        credentials: Credentials,
        cb_config: CircuitBreakerConfig,
    ) -> IntegrationResult<Self> {
        let mut client = Self::new(config, credentials)?;

        let cb = CircuitBreaker::new("sentinel_client".to_string(), cb_config);
        client.circuit_breaker = Some(Arc::new(cb));

        Ok(client)
    }

    pub async fn fetch_alerts_resilient(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<SentinelAlert>, CircuitBreakerError<IntegrationError>> {
        match &self.circuit_breaker {
            Some(cb) => cb.call(|| self.fetch_alerts(limit)).await,
            None => self
                .fetch_alerts(limit)
                .await
                .map_err(CircuitBreakerError::OperationFailed),
        }
    }

    pub async fn fetch_alerts_with_fallback(
        &self,
        limit: Option<usize>,
        cache: Arc<dyn AlertCache>,
    ) -> Result<Vec<SentinelAlert>, CircuitBreakerError<IntegrationError>> {
        match &self.circuit_breaker {
            Some(cb) => {
                cb.call_with_fallback(
                    || self.fetch_alerts(limit),
                    || cache.get_cached_alerts(),
                )
                .await
            }
            None => self
                .fetch_alerts(limit)
                .await
                .map_err(CircuitBreakerError::OperationFailed),
        }
    }
}
```

### Shield Client Integration

```rust
// src/integrations/shield/client.rs
impl ShieldClient {
    pub fn new_resilient(
        config: ConnectionConfig,
        credentials: Credentials,
    ) -> IntegrationResult<Self> {
        let mut client = Self::new(config, credentials)?;

        // Use aggressive config for security-critical service
        let cb_config = CircuitBreakerConfig::aggressive_preset();
        let cb = CircuitBreaker::new("shield_client".to_string(), cb_config);

        client.circuit_breaker = Some(Arc::new(cb));

        Ok(client)
    }

    pub async fn analyze_threat_resilient(
        &self,
        data: ThreatData,
    ) -> Result<ThreatAnalysis, CircuitBreakerError<IntegrationError>> {
        match &self.circuit_breaker {
            Some(cb) => cb.call(|| self.analyze_threat(data.clone())).await,
            None => self
                .analyze_threat(data)
                .await
                .map_err(CircuitBreakerError::OperationFailed),
        }
    }
}
```

### Edge Agent Client Integration (with streaming)

```rust
// src/integrations/edge_agent/client.rs
impl EdgeAgentClient {
    pub async fn stream_inferences_resilient(
        &self,
    ) -> Result<CircuitBreakerStream<InferenceStream, Inference, IntegrationError>, IntegrationError> {
        let stream = self.stream_inferences().await?;

        let cb = match &self.circuit_breaker {
            Some(cb) => Arc::clone(cb),
            None => {
                return Err(IntegrationError::Configuration(
                    "Circuit breaker not configured".to_string(),
                ))
            }
        };

        Ok(CircuitBreakerStream::new(stream, cb))
    }
}
```

## Registry-Based Integration

```rust
/// Global circuit breaker registry for the application
pub struct AppCircuitBreakers {
    registry: CircuitBreakerRegistry,
}

impl AppCircuitBreakers {
    pub fn new() -> Self {
        Self {
            registry: CircuitBreakerRegistry::new(),
        }
    }

    /// Initialize all circuit breakers from configuration
    pub fn initialize_from_config(&self, config: &AppConfig) -> Result<()> {
        // Sentinel client
        if let Some(cb_config) = &config.circuit_breakers.sentinel {
            self.registry.register(
                "sentinel_client",
                CircuitBreaker::new("sentinel_client".to_string(), cb_config.clone()),
            );
        }

        // Shield client
        if let Some(cb_config) = &config.circuit_breakers.shield {
            self.registry.register(
                "shield_client",
                CircuitBreaker::new("shield_client".to_string(), cb_config.clone()),
            );
        }

        // Edge agent client
        if let Some(cb_config) = &config.circuit_breakers.edge_agent {
            self.registry.register(
                "edge_agent_client",
                CircuitBreaker::new("edge_agent_client".to_string(), cb_config.clone()),
            );
        }

        // Database pool
        if let Some(cb_config) = &config.circuit_breakers.database {
            self.registry.register(
                "database_pool",
                CircuitBreaker::new("database_pool".to_string(), cb_config.clone()),
            );
        }

        Ok(())
    }

    /// Get circuit breaker for a service
    pub fn get(&self, name: &str) -> Option<Arc<CircuitBreaker>> {
        self.registry.get(name)
    }
}

// Global instance
lazy_static! {
    pub static ref APP_CIRCUIT_BREAKERS: AppCircuitBreakers = AppCircuitBreakers::new();
}
```

## Complete Integration Example

```rust
// Complete example: Incident Manager with circuit breakers

use llm_incident_manager::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::from_file("config.yaml")?;

    // Initialize circuit breakers
    APP_CIRCUIT_BREAKERS.initialize_from_config(&config)?;

    // Create Sentinel client with circuit breaker
    let sentinel_cb = APP_CIRCUIT_BREAKERS
        .get("sentinel_client")
        .expect("Sentinel circuit breaker not found");

    let sentinel_client = SentinelClient::new(
        config.integrations.sentinel.connection,
        config.integrations.sentinel.credentials,
    )?
    .with_circuit_breaker(sentinel_cb);

    // Create database pool with circuit breaker
    let db_pool = create_db_pool(&config.database.url).await?;
    let db_cb = APP_CIRCUIT_BREAKERS
        .get("database_pool")
        .expect("Database circuit breaker not found");
    let resilient_db = ResilientDbPool::new(db_pool, db_cb);

    // Process incidents with circuit breaker protection
    loop {
        // Fetch alerts with circuit breaker
        let alerts = match sentinel_client.fetch_alerts_resilient(Some(10)).await {
            Ok(alerts) => alerts,
            Err(CircuitBreakerError::CircuitOpen) => {
                warn!("Sentinel circuit breaker is open, using cached alerts");
                // Use fallback mechanism
                get_cached_alerts()
            }
            Err(e) => {
                error!("Failed to fetch alerts: {:?}", e);
                continue;
            }
        };

        // Store in database with circuit breaker
        for alert in alerts {
            resilient_db
                .execute(|pool| async move {
                    sqlx::query("INSERT INTO alerts (...) VALUES (...)")
                        .execute(&pool)
                        .await
                })
                .await?;
        }

        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
```

## Related Documents

- [State Machine Design](./CIRCUIT_BREAKER_STATE_MACHINE.md)
- [Core Architecture](./CIRCUIT_BREAKER_CORE_ARCHITECTURE.md)
- [Failure Detection Strategies](./CIRCUIT_BREAKER_FAILURE_DETECTION.md)
- [Configuration Schema](./CIRCUIT_BREAKER_CONFIGURATION_SCHEMA.md)
- [Fallback Mechanisms](./CIRCUIT_BREAKER_FALLBACK_MECHANISMS.md)
- [Testing Strategy](./CIRCUIT_BREAKER_TESTING_STRATEGY.md)
- [Migration Plan](./CIRCUIT_BREAKER_MIGRATION_PLAN.md)
