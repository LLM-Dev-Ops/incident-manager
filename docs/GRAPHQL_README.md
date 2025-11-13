# GraphQL API Documentation Index

## Overview

The LLM Incident Manager GraphQL API provides a modern, flexible, and type-safe interface for managing incidents, with support for real-time subscriptions and complex queries. This documentation suite covers everything from getting started to advanced implementation patterns.

## Quick Start

1. **Start the Server**: GraphQL API is available at `http://localhost:8080/graphql`
2. **Explore the Schema**: Use GraphQL Playground at `http://localhost:8080/graphql/playground`
3. **Run Your First Query**:
   ```graphql
   query {
     incidents(first: 10) {
       edges {
         node {
           id
           title
           severity
           status
         }
       }
     }
   }
   ```

## Documentation Structure

### 1. For API Users

#### [GraphQL API Guide](./GRAPHQL_API_GUIDE.md)
**Complete API usage guide** covering:
- Architecture overview
- Authentication methods
- Query and mutation syntax
- Subscription patterns
- Pagination strategies
- Error handling
- Rate limiting and query complexity
- Best practices

**Start here if**: You're consuming the API from a client application.

#### [GraphQL Schema Reference](./GRAPHQL_SCHEMA_REFERENCE.md)
**Complete schema documentation** including:
- All object types with field descriptions
- Input types and enums
- Query operations with examples
- Mutation operations with examples
- Subscription events
- Custom scalars and interfaces

**Start here if**: You need detailed reference for all available types and operations.

#### [GraphQL Examples](./GRAPHQL_EXAMPLES.md)
**Real-world query patterns** including:
- Basic CRUD operations
- Complex nested queries
- Pagination examples
- Advanced filtering
- Mutation patterns
- Subscription examples
- Analytics queries
- Dashboard use cases

**Start here if**: You want copy-paste examples for common scenarios.

### 2. For Client Integration

#### [GraphQL Integration Guide](./GRAPHQL_INTEGRATION_GUIDE.md)
**Client library integration** covering:
- Apollo Client setup and usage
- Relay configuration
- urql integration
- Plain fetch examples
- WebSocket subscriptions
- TypeScript type generation
- React/Vue integration
- Caching strategies

**Start here if**: You're integrating the API with a specific client library or framework.

### 3. For API Developers

#### [GraphQL Development Guide](./GRAPHQL_DEVELOPMENT_GUIDE.md)
**Implementation guide** covering:
- Project setup with async-graphql
- Adding new types
- Implementing queries
- Implementing mutations
- Implementing subscriptions
- DataLoader patterns for N+1 prevention
- Testing guidelines
- Performance optimization
- Security and authorization

**Start here if**: You're implementing or extending the GraphQL API.

## Additional Resources

### Architecture Documentation
- [GRAPHQL_ARCHITECTURE.md](./GRAPHQL_ARCHITECTURE.md) - System architecture and design patterns
- [GRAPHQL_ARCHITECTURE_DIAGRAM.md](./GRAPHQL_ARCHITECTURE_DIAGRAM.md) - Visual architecture diagrams

### Implementation Guides
- [GRAPHQL_IMPLEMENTATION_GUIDE.md](./GRAPHQL_IMPLEMENTATION_GUIDE.md) - Step-by-step implementation
- [GRAPHQL_TEST_SPECIFICATION.md](./GRAPHQL_TEST_SPECIFICATION.md) - Testing specifications

### Comparison Guides
- [GRAPHQL_VS_REST.md](./GRAPHQL_VS_REST.md) - GraphQL vs REST API comparison

## Key Features

### Type Safety
- Strongly-typed schema with introspection
- Automatic validation of queries and mutations
- IDE autocomplete and documentation

### Flexible Queries
- Request exactly the data you need
- Nest related data in a single query
- Avoid over-fetching or under-fetching

### Real-time Updates
- WebSocket-based subscriptions
- Live incident updates
- Event-driven notifications

### Performance
- DataLoader for N+1 query prevention
- Automatic request batching
- Query complexity analysis
- Built-in caching support

### Developer Experience
- GraphQL Playground for exploration
- Comprehensive error messages
- Self-documenting schema
- Code generation tools support

## Common Use Cases

### 1. Incident Dashboard
Query active incidents with filters, sorting, and pagination:
```graphql
query Dashboard {
  incidents(
    first: 20
    filter: { severity: [P0, P1], status: [NEW, ACKNOWLEDGED] }
    orderBy: { field: CREATED_AT, direction: DESC }
  ) {
    edges {
      node {
        id
        title
        severity
        status
        createdAt
        sla { resolutionDeadline }
      }
    }
  }
}
```

### 2. Real-time Monitoring
Subscribe to incident updates:
```graphql
subscription Monitor {
  incidentUpdated(filter: { severity: [P0, P1] }) {
    incident { id title status }
    updateType
  }
}
```

### 3. Analytics
Get incident metrics and trends:
```graphql
query Analytics {
  analytics(timeRange: { start: "2025-11-01T00:00:00Z", end: "2025-11-12T23:59:59Z" }) {
    totalIncidents
    performance { averageMttr p95Mttr }
    slaMetrics { resolutionCompliance }
  }
}
```

## API Endpoints

| Endpoint | Protocol | Purpose |
|----------|----------|---------|
| `POST /graphql` | HTTP | Queries and mutations |
| `GET /graphql` | WebSocket | Real-time subscriptions |
| `GET /graphql/playground` | HTTP | Interactive API explorer |
| `GET /graphql/schema` | HTTP | Schema introspection |

## Authentication

Include your API key in the Authorization header:

```bash
curl -X POST http://localhost:8080/graphql \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ incidents(first: 10) { totalCount } }"}'
```

For WebSocket subscriptions:
```javascript
const client = new WebSocket('ws://localhost:8080/graphql', {
  connectionParams: {
    authorization: 'Bearer YOUR_API_KEY'
  }
});
```

## Rate Limits

| Tier | Requests/Hour | Concurrent Connections | Max Complexity |
|------|---------------|------------------------|----------------|
| Free | 1,000 | 5 | 1,000 |
| Pro | 10,000 | 20 | 10,000 |
| Enterprise | 100,000 | 100 | 100,000 |

## Getting Help

- **Issues**: Report bugs on [GitHub Issues](https://github.com/globalbusinessadvisors/llm-incident-manager/issues)
- **Discussions**: Ask questions on [GitHub Discussions](https://github.com/globalbusinessadvisors/llm-incident-manager/discussions)
- **Support**: Email support@example.com
- **Main README**: See [../README.md](../README.md) for general project information

## Version

Current API Version: **1.0.0**

GraphQL API Status: **Documentation Complete** (Implementation Pending)

## Next Steps

1. **New to GraphQL?** Start with [GraphQL API Guide](./GRAPHQL_API_GUIDE.md)
2. **Need examples?** Check [GraphQL Examples](./GRAPHQL_EXAMPLES.md)
3. **Integrating a client?** Read [GraphQL Integration Guide](./GRAPHQL_INTEGRATION_GUIDE.md)
4. **Implementing the API?** Follow [GraphQL Development Guide](./GRAPHQL_DEVELOPMENT_GUIDE.md)
5. **Need full reference?** Browse [GraphQL Schema Reference](./GRAPHQL_SCHEMA_REFERENCE.md)

---

**Note**: The GraphQL API documentation is complete and ready for implementation. The actual GraphQL server implementation using async-graphql in Rust is pending and can be built following the [GraphQL Development Guide](./GRAPHQL_DEVELOPMENT_GUIDE.md).
