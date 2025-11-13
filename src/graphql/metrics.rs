//! Metrics integration for GraphQL
//!
//! Tracks query execution time, complexity, and errors

use async_graphql::*;
use async_graphql::extensions::{Extension, ExtensionContext, ExtensionFactory, NextExecute, NextParseQuery, NextResolve, NextValidation, ResolveInfo};
use async_graphql::parser::types::ExecutableDocument;
use std::sync::Arc;
use tracing::{error, info, warn};

/// GraphQL metrics extension
pub struct MetricsExtension;

impl ExtensionFactory for MetricsExtension {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(MetricsExtensionImpl)
    }
}

struct MetricsExtensionImpl;

#[async_trait::async_trait]
impl Extension for MetricsExtensionImpl {
    /// Called when parsing the query
    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &str,
        variables: &Variables,
        next: NextParseQuery<'_>,
    ) -> ServerResult<ExecutableDocument> {
        let start = std::time::Instant::now();
        let result = next.run(ctx, query, variables).await;
        let duration = start.elapsed();

        if result.is_err() {
            warn!(
                query_length = query.len(),
                duration_ms = duration.as_millis(),
                "GraphQL query parse failed"
            );
        }

        result
    }

    /// Called when validating the query
    async fn validation(
        &self,
        ctx: &ExtensionContext<'_>,
        next: NextValidation<'_>,
    ) -> Result<ValidationResult, Vec<ServerError>> {
        let start = std::time::Instant::now();
        let result = next.run(ctx).await;
        let duration = start.elapsed();

        match &result {
            Ok(_) => {
                info!(duration_ms = duration.as_millis(), "GraphQL query validated");
            }
            Err(errors) => {
                warn!(
                    duration_ms = duration.as_millis(),
                    error_count = errors.len(),
                    "GraphQL query validation failed"
                );
            }
        }

        result
    }

    /// Called when executing the query
    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        let start = std::time::Instant::now();
        let response = next.run(ctx, operation_name).await;
        let duration = start.elapsed();

        // Log execution metrics
        let error_count = response.errors.len();
        let has_data = !matches!(response.data, async_graphql::Value::Null);

        if error_count > 0 {
            error!(
                operation = operation_name,
                duration_ms = duration.as_millis(),
                error_count = error_count,
                has_data = has_data,
                "GraphQL query execution completed with errors"
            );
        } else {
            info!(
                operation = operation_name,
                duration_ms = duration.as_millis(),
                has_data = has_data,
                "GraphQL query execution completed successfully"
            );
        }

        // Track metrics if available
        if let Err(e) = track_graphql_metrics(operation_name, duration, error_count) {
            warn!("Failed to track GraphQL metrics: {}", e);
        }

        response
    }

    /// Called when resolving a field
    async fn resolve(
        &self,
        ctx: &ExtensionContext<'_>,
        info: ResolveInfo<'_>,
        next: NextResolve<'_>,
    ) -> ServerResult<Option<Value>> {
        let start = std::time::Instant::now();
        let field_name = info.path_node.field_name().to_string();
        let parent_type = info.parent_type.to_string();
        let result = next.run(ctx, info).await;
        let duration = start.elapsed();

        // Log slow field resolvers (> 100ms)
        if duration.as_millis() > 100 {
            warn!(
                field = field_name,
                parent_type = parent_type,
                duration_ms = duration.as_millis(),
                "Slow GraphQL field resolver"
            );
        }

        result
    }
}

/// Track GraphQL metrics using Prometheus
fn track_graphql_metrics(
    operation: Option<&str>,
    duration: std::time::Duration,
    error_count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::metrics::{GRAPHQL_QUERIES_TOTAL, GRAPHQL_QUERY_DURATION, GRAPHQL_ERRORS_TOTAL};

    let operation_label = operation.unwrap_or("unknown");

    // Increment query counter
    GRAPHQL_QUERIES_TOTAL
        .with_label_values(&[operation_label])
        .inc();

    // Record query duration
    GRAPHQL_QUERY_DURATION
        .with_label_values(&[operation_label])
        .observe(duration.as_secs_f64());

    // Increment error counter if there were errors
    if error_count > 0 {
        GRAPHQL_ERRORS_TOTAL
            .with_label_values(&[operation_label])
            .inc_by(error_count as f64);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_extension_creation() {
        let factory = MetricsExtension;
        let extension = factory.create();
        assert!(!std::ptr::eq(Arc::as_ptr(&extension), std::ptr::null()));
    }
}
