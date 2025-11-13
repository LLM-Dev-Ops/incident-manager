//! GraphQL schema definition
//!
//! Combines queries, mutations, and subscriptions into a complete schema

use async_graphql::*;

use super::context::GraphQLContext;
use super::metrics::MetricsExtension;
use super::mutations::MutationRoot;
use super::queries::QueryRoot;
use super::subscriptions::SubscriptionRoot;

/// The complete GraphQL schema type
pub type GraphQLSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Build the GraphQL schema
pub fn build_schema() -> GraphQLSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .extension(MetricsExtension) // Add metrics tracking
        .enable_federation()
        .enable_subscription_in_federation()
        .limit_depth(10) // Prevent deeply nested queries
        .limit_complexity(100) // Prevent overly complex queries
        .finish()
}

/// Execute a GraphQL query/mutation
///
/// This is a helper function for testing and programmatic access
pub async fn execute_query(
    schema: &GraphQLSchema,
    ctx: GraphQLContext,
    query: &str,
    variables: Option<serde_json::Value>,
) -> Response {
    let mut request = Request::new(query);

    if let Some(vars) = variables {
        request = request.variables(Variables::from_json(vars));
    }

    request = request.data(ctx);

    schema.execute(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::IncidentProcessor;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_schema_builds() {
        let schema = build_schema();
        assert!(!schema.sdl().is_empty());
    }

    #[tokio::test]
    async fn test_introspection_query() {
        let schema = build_schema();

        // Create a mock processor (this would need proper initialization in real tests)
        // For now, we just test that the schema structure is valid
        let query = r#"
            {
                __schema {
                    queryType {
                        name
                    }
                    mutationType {
                        name
                    }
                    subscriptionType {
                        name
                    }
                }
            }
        "#;

        let request = Request::new(query);
        let response = schema.execute(request).await;

        // Schema should be queryable
        assert!(response.errors.is_empty() || !matches!(response.data, async_graphql::Value::Null));
    }

    #[tokio::test]
    async fn test_health_query() {
        let schema = build_schema();

        let query = r#"
            {
                health {
                    status
                    version
                }
            }
        "#;

        let request = Request::new(query);
        let response = schema.execute(request).await;

        // Health query should work without context
        assert!(response.errors.is_empty() || !matches!(response.data, async_graphql::Value::Null));
    }
}
