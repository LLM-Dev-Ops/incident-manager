# GraphQL Integration Guide

## Client Integration Examples

This guide provides comprehensive examples for integrating with the LLM Incident Manager GraphQL API using various client libraries and frameworks.

## Table of Contents

- [Apollo Client](#apollo-client)
- [Relay](#relay)
- [urql](#urql)
- [Plain Fetch](#plain-fetch)
- [WebSocket Subscriptions](#websocket-subscriptions)
- [TypeScript Integration](#typescript-integration)
- [React Integration](#react-integration)
- [Vue Integration](#vue-integration)
- [Caching Strategies](#caching-strategies)
- [Error Handling](#error-handling)
- [Best Practices](#best-practices)

## Apollo Client

### Installation

```bash
npm install @apollo/client graphql
# or
yarn add @apollo/client graphql
```

### Basic Setup

```typescript
import { ApolloClient, InMemoryCache, HttpLink, split } from '@apollo/client';
import { GraphQLWsLink } from '@apollo/client/link/subscriptions';
import { getMainDefinition } from '@apollo/client/utilities';
import { createClient } from 'graphql-ws';

// HTTP link for queries and mutations
const httpLink = new HttpLink({
  uri: 'http://localhost:8080/graphql',
  headers: {
    authorization: `Bearer ${process.env.API_KEY}`,
  },
});

// WebSocket link for subscriptions
const wsLink = new GraphQLWsLink(
  createClient({
    url: 'ws://localhost:8080/graphql',
    connectionParams: {
      authorization: `Bearer ${process.env.API_KEY}`,
    },
  })
);

// Split traffic based on operation type
const splitLink = split(
  ({ query }) => {
    const definition = getMainDefinition(query);
    return (
      definition.kind === 'OperationDefinition' &&
      definition.operation === 'subscription'
    );
  },
  wsLink,
  httpLink
);

// Create Apollo Client instance
const client = new ApolloClient({
  link: splitLink,
  cache: new InMemoryCache({
    typePolicies: {
      Query: {
        fields: {
          incidents: {
            keyArgs: ['filter', 'orderBy'],
            merge(existing, incoming, { args }) {
              if (!existing) return incoming;

              // Handle pagination merge
              const { edges, pageInfo } = incoming;
              return {
                ...incoming,
                edges: [...(existing.edges || []), ...edges],
                pageInfo,
              };
            },
          },
        },
      },
      Incident: {
        fields: {
          relatedIncidents: {
            merge(existing, incoming) {
              return incoming;
            },
          },
        },
      },
    },
  }),
});

export default client;
```

### Query Example

```typescript
import { gql, useQuery } from '@apollo/client';

const GET_INCIDENTS = gql`
  query GetIncidents(
    $first: Int!
    $after: String
    $filter: IncidentFilterInput
  ) {
    incidents(first: $first, after: $after, filter: $filter) {
      edges {
        cursor
        node {
          id
          title
          severity
          status
          createdAt
          assignedTo {
            id
            name
            email
          }
        }
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
`;

function IncidentList() {
  const { loading, error, data, fetchMore } = useQuery(GET_INCIDENTS, {
    variables: {
      first: 20,
      filter: {
        status: ['NEW', 'ACKNOWLEDGED'],
        severity: ['P0', 'P1'],
      },
    },
    pollInterval: 30000, // Poll every 30 seconds
  });

  if (loading) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;

  const loadMore = () => {
    if (data?.incidents?.pageInfo?.hasNextPage) {
      fetchMore({
        variables: {
          after: data.incidents.pageInfo.endCursor,
        },
      });
    }
  };

  return (
    <div>
      <h2>Incidents ({data.incidents.totalCount})</h2>
      {data.incidents.edges.map(({ node }) => (
        <div key={node.id}>
          <h3>{node.title}</h3>
          <p>Severity: {node.severity}</p>
          <p>Status: {node.status}</p>
          {node.assignedTo && <p>Assigned: {node.assignedTo.name}</p>}
        </div>
      ))}
      {data.incidents.pageInfo.hasNextPage && (
        <button onClick={loadMore}>Load More</button>
      )}
    </div>
  );
}
```

### Mutation Example

```typescript
import { gql, useMutation } from '@apollo/client';

const ACKNOWLEDGE_INCIDENT = gql`
  mutation AcknowledgeIncident(
    $incidentId: ID!
    $actor: String!
    $notes: String
  ) {
    acknowledgeIncident(
      incidentId: $incidentId
      actor: $actor
      notes: $notes
    ) {
      incident {
        id
        status
        acknowledgedAt
      }
      success
      message
    }
  }
`;

function AcknowledgeButton({ incidentId }) {
  const [acknowledge, { loading, error }] = useMutation(
    ACKNOWLEDGE_INCIDENT,
    {
      variables: {
        incidentId,
        actor: 'user@example.com',
        notes: 'Investigating the issue',
      },
      // Update cache after mutation
      update(cache, { data: { acknowledgeIncident } }) {
        if (acknowledgeIncident.success) {
          cache.modify({
            id: cache.identify({ __typename: 'Incident', id: incidentId }),
            fields: {
              status() {
                return 'ACKNOWLEDGED';
              },
              acknowledgedAt() {
                return acknowledgeIncident.incident.acknowledgedAt;
              },
            },
          });
        }
      },
      // Optimistic response
      optimisticResponse: {
        acknowledgeIncident: {
          __typename: 'AcknowledgeIncidentResponse',
          success: true,
          message: 'Acknowledging...',
          incident: {
            __typename: 'Incident',
            id: incidentId,
            status: 'ACKNOWLEDGED',
            acknowledgedAt: new Date().toISOString(),
          },
        },
      },
    }
  );

  return (
    <button onClick={() => acknowledge()} disabled={loading}>
      {loading ? 'Acknowledging...' : 'Acknowledge'}
    </button>
  );
}
```

### Subscription Example

```typescript
import { gql, useSubscription } from '@apollo/client';

const INCIDENT_UPDATES = gql`
  subscription IncidentUpdates($filter: IncidentFilterInput) {
    incidentUpdated(filter: $filter) {
      incident {
        id
        title
        severity
        status
        updatedAt
      }
      updateType
      changedFields
      actor {
        name
      }
    }
  }
`;

function IncidentNotifications() {
  const { data, loading, error } = useSubscription(INCIDENT_UPDATES, {
    variables: {
      filter: {
        severity: ['P0', 'P1'],
        environment: ['PRODUCTION'],
      },
    },
    onSubscriptionData: ({ subscriptionData }) => {
      const update = subscriptionData.data.incidentUpdated;
      console.log('Incident updated:', update.incident.title);

      // Show notification
      if (update.updateType === 'CREATED') {
        showNotification(`New ${update.incident.severity} incident: ${update.incident.title}`);
      }
    },
  });

  if (loading) return <div>Connecting...</div>;
  if (error) return <div>Connection error: {error.message}</div>;

  return (
    <div>
      <h3>Real-time Updates</h3>
      {data && (
        <div>
          Last update: {data.incidentUpdated.incident.title} - {data.incidentUpdated.updateType}
        </div>
      )}
    </div>
  );
}
```

## Relay

### Installation

```bash
npm install react-relay relay-runtime
npm install --save-dev relay-compiler babel-plugin-relay
```

### Relay Configuration (relay.config.js)

```javascript
module.exports = {
  src: './src',
  schema: './schema.graphql',
  exclude: ['**/node_modules/**', '**/__generated__/**'],
};
```

### Environment Setup

```typescript
import {
  Environment,
  Network,
  RecordSource,
  Store,
} from 'relay-runtime';

async function fetchQuery(operation, variables) {
  const response = await fetch('http://localhost:8080/graphql', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${process.env.API_KEY}`,
    },
    body: JSON.stringify({
      query: operation.text,
      variables,
    }),
  });
  return response.json();
}

const environment = new Environment({
  network: Network.create(fetchQuery),
  store: new Store(new RecordSource()),
});

export default environment;
```

### Query with Relay

```typescript
import { graphql, useLazyLoadQuery } from 'react-relay';

const IncidentListQuery = graphql`
  query IncidentListQuery(
    $first: Int!
    $after: String
    $filter: IncidentFilterInput
  ) {
    incidents(first: $first, after: $after, filter: $filter)
      @connection(key: "IncidentList_incidents") {
      edges {
        node {
          id
          title
          severity
          status
          createdAt
        }
      }
    }
  }
`;

function IncidentList() {
  const data = useLazyLoadQuery(IncidentListQuery, {
    first: 20,
    filter: {
      status: ['NEW', 'ACKNOWLEDGED'],
    },
  });

  return (
    <div>
      {data.incidents.edges.map(({ node }) => (
        <div key={node.id}>
          <h3>{node.title}</h3>
          <p>{node.severity}</p>
        </div>
      ))}
    </div>
  );
}
```

### Mutation with Relay

```typescript
import { graphql, useMutation } from 'react-relay';

const AcknowledgeIncidentMutation = graphql`
  mutation AcknowledgeIncidentMutation(
    $incidentId: ID!
    $actor: String!
    $notes: String
  ) {
    acknowledgeIncident(
      incidentId: $incidentId
      actor: $actor
      notes: $notes
    ) {
      incident {
        id
        status
        acknowledgedAt
      }
      success
      message
    }
  }
`;

function AcknowledgeButton({ incidentId }) {
  const [commit, isInFlight] = useMutation(AcknowledgeIncidentMutation);

  const handleAcknowledge = () => {
    commit({
      variables: {
        incidentId,
        actor: 'user@example.com',
        notes: 'Looking into this',
      },
      onCompleted: (response) => {
        if (response.acknowledgeIncident.success) {
          console.log('Incident acknowledged');
        }
      },
      onError: (error) => {
        console.error('Error:', error);
      },
    });
  };

  return (
    <button onClick={handleAcknowledge} disabled={isInFlight}>
      {isInFlight ? 'Acknowledging...' : 'Acknowledge'}
    </button>
  );
}
```

## urql

### Installation

```bash
npm install urql graphql
```

### Client Setup

```typescript
import { createClient, defaultExchanges, subscriptionExchange } from 'urql';
import { createClient as createWSClient } from 'graphql-ws';

const wsClient = createWSClient({
  url: 'ws://localhost:8080/graphql',
  connectionParams: {
    authorization: `Bearer ${process.env.API_KEY}`,
  },
});

const client = createClient({
  url: 'http://localhost:8080/graphql',
  fetchOptions: {
    headers: {
      authorization: `Bearer ${process.env.API_KEY}`,
    },
  },
  exchanges: [
    ...defaultExchanges,
    subscriptionExchange({
      forwardSubscription: (operation) => ({
        subscribe: (sink) => ({
          unsubscribe: wsClient.subscribe(operation, sink),
        }),
      }),
    }),
  ],
});

export default client;
```

### Query with urql

```typescript
import { useQuery } from 'urql';

const GET_INCIDENTS = `
  query GetIncidents($first: Int!, $filter: IncidentFilterInput) {
    incidents(first: $first, filter: $filter) {
      edges {
        node {
          id
          title
          severity
          status
        }
      }
      totalCount
    }
  }
`;

function IncidentList() {
  const [result, reexecuteQuery] = useQuery({
    query: GET_INCIDENTS,
    variables: {
      first: 20,
      filter: {
        status: ['NEW', 'ACKNOWLEDGED'],
      },
    },
  });

  const { data, fetching, error } = result;

  if (fetching) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <div>
      <button onClick={() => reexecuteQuery({ requestPolicy: 'network-only' })}>
        Refresh
      </button>
      {data.incidents.edges.map(({ node }) => (
        <div key={node.id}>
          <h3>{node.title}</h3>
        </div>
      ))}
    </div>
  );
}
```

### Mutation with urql

```typescript
import { useMutation } from 'urql';

const ACKNOWLEDGE_INCIDENT = `
  mutation AcknowledgeIncident($incidentId: ID!, $actor: String!) {
    acknowledgeIncident(incidentId: $incidentId, actor: $actor) {
      success
      incident {
        id
        status
      }
    }
  }
`;

function AcknowledgeButton({ incidentId }) {
  const [result, executeMutation] = useMutation(ACKNOWLEDGE_INCIDENT);

  const handleClick = () => {
    executeMutation({ incidentId, actor: 'user@example.com' });
  };

  return (
    <button onClick={handleClick} disabled={result.fetching}>
      {result.fetching ? 'Acknowledging...' : 'Acknowledge'}
    </button>
  );
}
```

### Subscription with urql

```typescript
import { useSubscription } from 'urql';

const INCIDENT_UPDATES = `
  subscription IncidentUpdates {
    incidentUpdated {
      incident {
        id
        title
        status
      }
      updateType
    }
  }
`;

function IncidentNotifications() {
  const [result] = useSubscription({ query: INCIDENT_UPDATES });

  if (!result.data) return <div>Waiting for updates...</div>;

  return (
    <div>
      <h3>Latest Update</h3>
      <p>{result.data.incidentUpdated.incident.title}</p>
      <p>Type: {result.data.incidentUpdated.updateType}</p>
    </div>
  );
}
```

## Plain Fetch

### Query Example

```typescript
async function queryIncidents(filter: any) {
  const response = await fetch('http://localhost:8080/graphql', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${process.env.API_KEY}`,
    },
    body: JSON.stringify({
      query: `
        query GetIncidents($filter: IncidentFilterInput) {
          incidents(first: 20, filter: $filter) {
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
      `,
      variables: { filter },
    }),
  });

  const { data, errors } = await response.json();

  if (errors) {
    throw new Error(errors[0].message);
  }

  return data.incidents;
}

// Usage
const incidents = await queryIncidents({
  severity: ['P0', 'P1'],
  status: ['NEW'],
});
```

### Mutation Example

```typescript
async function acknowledgeIncident(incidentId: string, actor: string) {
  const response = await fetch('http://localhost:8080/graphql', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${process.env.API_KEY}`,
    },
    body: JSON.stringify({
      query: `
        mutation AcknowledgeIncident($incidentId: ID!, $actor: String!) {
          acknowledgeIncident(incidentId: $incidentId, actor: $actor) {
            success
            message
            incident {
              id
              status
              acknowledgedAt
            }
          }
        }
      `,
      variables: { incidentId, actor },
    }),
  });

  const { data, errors } = await response.json();

  if (errors) {
    throw new Error(errors[0].message);
  }

  return data.acknowledgeIncident;
}
```

## WebSocket Subscriptions

### Using graphql-ws

```typescript
import { createClient } from 'graphql-ws';
import WebSocket from 'ws';

const client = createClient({
  url: 'ws://localhost:8080/graphql',
  webSocketImpl: WebSocket,
  connectionParams: {
    authorization: `Bearer ${process.env.API_KEY}`,
  },
});

// Subscribe to incident updates
const unsubscribe = client.subscribe(
  {
    query: `
      subscription IncidentUpdates {
        incidentUpdated {
          incident {
            id
            title
            severity
            status
          }
          updateType
        }
      }
    `,
  },
  {
    next: (data) => {
      console.log('Incident update:', data);
    },
    error: (error) => {
      console.error('Subscription error:', error);
    },
    complete: () => {
      console.log('Subscription completed');
    },
  }
);

// Unsubscribe when done
// unsubscribe();
```

### Using WebSocket Directly

```typescript
const ws = new WebSocket('ws://localhost:8080/graphql', 'graphql-ws');

ws.on('open', () => {
  // Send connection init
  ws.send(JSON.stringify({
    type: 'connection_init',
    payload: {
      authorization: `Bearer ${process.env.API_KEY}`,
    },
  }));
});

ws.on('message', (data) => {
  const message = JSON.parse(data.toString());

  switch (message.type) {
    case 'connection_ack':
      console.log('Connection acknowledged');

      // Subscribe to updates
      ws.send(JSON.stringify({
        id: '1',
        type: 'start',
        payload: {
          query: `
            subscription {
              incidentCreated(severity: [P0, P1]) {
                id
                title
                severity
              }
            }
          `,
        },
      }));
      break;

    case 'data':
      console.log('Received data:', message.payload);
      break;

    case 'error':
      console.error('Error:', message.payload);
      break;
  }
});
```

## TypeScript Integration

### Generate Types

```bash
# Install codegen
npm install --save-dev @graphql-codegen/cli @graphql-codegen/typescript @graphql-codegen/typescript-operations @graphql-codegen/typescript-react-apollo

# Create codegen.yml
cat > codegen.yml << EOF
schema: http://localhost:8080/graphql
documents: './src/**/*.graphql'
generates:
  ./src/generated/graphql.ts:
    plugins:
      - typescript
      - typescript-operations
      - typescript-react-apollo
    config:
      withHooks: true
      withComponent: false
EOF

# Generate types
npx graphql-codegen
```

### Usage with Generated Types

```typescript
import { useGetIncidentsQuery, useAcknowledgeIncidentMutation } from './generated/graphql';

function IncidentList() {
  const { data, loading, error } = useGetIncidentsQuery({
    variables: {
      first: 20,
      filter: {
        severity: ['P0', 'P1'],
      },
    },
  });

  const [acknowledge] = useAcknowledgeIncidentMutation();

  if (loading) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <div>
      {data?.incidents?.edges.map(({ node }) => (
        <div key={node.id}>
          <h3>{node.title}</h3>
          <button
            onClick={() =>
              acknowledge({
                variables: {
                  incidentId: node.id,
                  actor: 'user@example.com',
                },
              })
            }
          >
            Acknowledge
          </button>
        </div>
      ))}
    </div>
  );
}
```

## React Integration

### Context Provider

```typescript
import { ApolloProvider } from '@apollo/client';
import client from './apolloClient';

function App() {
  return (
    <ApolloProvider client={client}>
      <IncidentDashboard />
    </ApolloProvider>
  );
}
```

### Custom Hooks

```typescript
import { gql, useQuery, useMutation } from '@apollo/client';

const GET_INCIDENT = gql`
  query GetIncident($id: ID!) {
    incident(id: $id) {
      id
      title
      severity
      status
      description
    }
  }
`;

export function useIncident(id: string) {
  return useQuery(GET_INCIDENT, {
    variables: { id },
    skip: !id,
  });
}

const RESOLVE_INCIDENT = gql`
  mutation ResolveIncident($input: ResolveIncidentInput!) {
    resolveIncident(input: $input) {
      success
      incident {
        id
        status
        resolvedAt
      }
    }
  }
`;

export function useResolveIncident() {
  return useMutation(RESOLVE_INCIDENT);
}
```

## Vue Integration

### Setup

```typescript
import { createApp, provide, h } from 'vue';
import { DefaultApolloClient } from '@vue/apollo-composable';
import client from './apolloClient';

const app = createApp({
  setup() {
    provide(DefaultApolloClient, client);
  },
  render: () => h(App),
});

app.mount('#app');
```

### Component Usage

```vue
<template>
  <div>
    <div v-if="loading">Loading...</div>
    <div v-else-if="error">Error: {{ error.message }}</div>
    <div v-else>
      <div v-for="edge in result?.incidents?.edges" :key="edge.node.id">
        <h3>{{ edge.node.title }}</h3>
        <p>{{ edge.node.severity }}</p>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import { useQuery, useMutation } from '@vue/apollo-composable';
import gql from 'graphql-tag';

const GET_INCIDENTS = gql`
  query GetIncidents {
    incidents(first: 20) {
      edges {
        node {
          id
          title
          severity
        }
      }
    }
  }
`;

export default defineComponent({
  setup() {
    const { result, loading, error } = useQuery(GET_INCIDENTS);

    return {
      result,
      loading,
      error,
    };
  },
});
</script>
```

## Caching Strategies

### Apollo Cache Configuration

```typescript
import { InMemoryCache } from '@apollo/client';

const cache = new InMemoryCache({
  typePolicies: {
    Query: {
      fields: {
        incidents: {
          // Custom cache key
          keyArgs: ['filter', 'orderBy'],

          // Merge strategy
          merge(existing, incoming, { args }) {
            const merged = existing ? { ...existing } : { edges: [], pageInfo: {} };

            if (args?.after) {
              // Pagination: append new edges
              merged.edges = [...merged.edges, ...incoming.edges];
            } else {
              // Fresh query: replace edges
              merged.edges = incoming.edges;
            }

            merged.pageInfo = incoming.pageInfo;
            merged.totalCount = incoming.totalCount;

            return merged;
          },
        },
      },
    },
    Incident: {
      fields: {
        // Custom read function
        isOverdue: {
          read(_, { readField }) {
            const deadline = readField('sla.resolutionDeadline');
            return deadline && new Date(deadline) < new Date();
          },
        },
      },
    },
  },
});
```

### Cache Persistence

```typescript
import { persistCache, LocalStorageWrapper } from 'apollo3-cache-persist';

async function setupCache() {
  const cache = new InMemoryCache();

  await persistCache({
    cache,
    storage: new LocalStorageWrapper(window.localStorage),
    maxSize: 1048576, // 1 MB
    debounce: 1000,
  });

  return cache;
}
```

## Error Handling

### Centralized Error Handler

```typescript
import { onError } from '@apollo/client/link/error';

const errorLink = onError(({ graphQLErrors, networkError, operation }) => {
  if (graphQLErrors) {
    graphQLErrors.forEach(({ message, extensions, path }) => {
      console.error(
        `[GraphQL error]: Message: ${message}, Code: ${extensions?.code}, Path: ${path}`
      );

      // Handle specific error codes
      switch (extensions?.code) {
        case 'UNAUTHENTICATED':
          // Redirect to login
          window.location.href = '/login';
          break;

        case 'RATE_LIMITED':
          // Show rate limit message
          const retryAfter = extensions.retryAfter;
          console.log(`Rate limited. Retry after ${retryAfter}s`);
          break;

        case 'NOT_FOUND':
          // Handle not found
          break;
      }
    });
  }

  if (networkError) {
    console.error(`[Network error]: ${networkError}`);
  }
});
```

### Retry Logic

```typescript
import { RetryLink } from '@apollo/client/link/retry';

const retryLink = new RetryLink({
  delay: {
    initial: 300,
    max: 10000,
    jitter: true,
  },
  attempts: {
    max: 3,
    retryIf: (error, operation) => {
      // Retry on network errors or 5xx status codes
      return !!error && error.statusCode >= 500;
    },
  },
});
```

## Best Practices

### 1. Fragment Composition

```typescript
const INCIDENT_FIELDS = gql`
  fragment IncidentFields on Incident {
    id
    title
    severity
    status
    createdAt
  }
`;

const GET_INCIDENTS = gql`
  ${INCIDENT_FIELDS}

  query GetIncidents {
    incidents(first: 20) {
      edges {
        node {
          ...IncidentFields
          assignedTo {
            id
            name
          }
        }
      }
    }
  }
`;
```

### 2. Request Batching

```typescript
import { BatchHttpLink } from '@apollo/client/link/batch-http';

const batchLink = new BatchHttpLink({
  uri: 'http://localhost:8080/graphql',
  batchMax: 10,
  batchInterval: 20,
});
```

### 3. Optimistic Updates

```typescript
const [acknowledge] = useMutation(ACKNOWLEDGE_INCIDENT, {
  optimisticResponse: {
    acknowledgeIncident: {
      __typename: 'AcknowledgeIncidentResponse',
      success: true,
      incident: {
        __typename: 'Incident',
        id: incidentId,
        status: 'ACKNOWLEDGED',
        acknowledgedAt: new Date().toISOString(),
      },
    },
  },
});
```

### 4. Query Deduplication

Apollo Client automatically deduplicates identical queries in flight:

```typescript
// These will only trigger one network request
const { data: data1 } = useQuery(GET_INCIDENT, { variables: { id: '123' } });
const { data: data2 } = useQuery(GET_INCIDENT, { variables: { id: '123' } });
```

### 5. Lazy Queries

```typescript
const [getIncident, { loading, data }] = useLazyQuery(GET_INCIDENT);

// Call when needed
<button onClick={() => getIncident({ variables: { id: '123' } })}>
  Load Incident
</button>
```

## Performance Tips

1. **Use fragments** to avoid duplicate field definitions
2. **Enable request batching** for multiple simultaneous queries
3. **Implement pagination** for large datasets
4. **Cache responses** aggressively
5. **Use optimistic updates** for better UX
6. **Lazy load** queries when possible
7. **Monitor bundle size** - tree-shake unused operations
8. **Use persisted queries** in production

## Further Reading

- [GraphQL API Guide](./GRAPHQL_API_GUIDE.md)
- [GraphQL Schema Reference](./GRAPHQL_SCHEMA_REFERENCE.md)
- [GraphQL Development Guide](./GRAPHQL_DEVELOPMENT_GUIDE.md)
- [GraphQL Examples](./GRAPHQL_EXAMPLES.md)
