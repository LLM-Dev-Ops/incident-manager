# WebSocket Client Integration Guide

## Overview

This guide provides practical examples for integrating with the LLM Incident Manager's WebSocket streaming API across multiple programming languages and frameworks.

## Table of Contents

- [JavaScript/TypeScript](#javascripttypescript)
  - [Browser (Native WebSocket)](#browser-native-websocket)
  - [Node.js (ws library)](#nodejs-ws-library)
  - [graphql-ws Client](#graphql-ws-client)
  - [Apollo Client](#apollo-client)
- [Python](#python)
  - [websockets Library](#websockets-library)
  - [gql Client](#gql-client)
- [Rust](#rust)
  - [tokio-tungstenite](#tokio-tungstenite)
  - [graphql-client](#graphql-client)
- [Go](#go)
  - [gorilla/websocket](#gorillawebsocket)
- [Common Patterns](#common-patterns)
  - [Authentication](#authentication)
  - [Reconnection Strategy](#reconnection-strategy)
  - [Error Handling](#error-handling)
  - [State Management](#state-management)

---

## JavaScript/TypeScript

### Browser (Native WebSocket)

#### Basic Connection

```typescript
interface WebSocketMessage {
  type: string;
  id?: string;
  payload?: any;
}

class IncidentStreamClient {
  private ws: WebSocket | null = null;
  private url: string;
  private token: string;
  private subscriptions: Map<string, (data: any) => void> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;

  constructor(url: string, token: string) {
    this.url = url;
    this.token = token;
  }

  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url, 'graphql-transport-ws');

      this.ws.onopen = () => {
        console.log('WebSocket connected');
        this.reconnectAttempts = 0;

        // Initialize connection with auth
        this.send({
          type: 'connection_init',
          payload: {
            Authorization: `Bearer ${this.token}`
          }
        });
      };

      this.ws.onmessage = (event) => {
        const message: WebSocketMessage = JSON.parse(event.data);
        this.handleMessage(message);

        if (message.type === 'connection_ack') {
          resolve();
        }
      };

      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        reject(error);
      };

      this.ws.onclose = () => {
        console.log('WebSocket closed');
        this.handleReconnect();
      };
    });
  }

  private send(message: WebSocketMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  private handleMessage(message: WebSocketMessage): void {
    switch (message.type) {
      case 'connection_ack':
        console.log('Connection acknowledged');
        break;

      case 'next':
        if (message.id) {
          const handler = this.subscriptions.get(message.id);
          if (handler && message.payload?.data) {
            handler(message.payload.data);
          }
        }
        break;

      case 'error':
        console.error('Subscription error:', message.payload);
        break;

      case 'complete':
        if (message.id) {
          this.subscriptions.delete(message.id);
          console.log(`Subscription ${message.id} completed`);
        }
        break;

      case 'pong':
        // Handle pong response
        break;
    }
  }

  subscribe(
    id: string,
    query: string,
    variables: Record<string, any> = {},
    handler: (data: any) => void
  ): void {
    this.subscriptions.set(id, handler);

    this.send({
      id,
      type: 'subscribe',
      payload: {
        query,
        variables
      }
    });
  }

  unsubscribe(id: string): void {
    this.send({
      id,
      type: 'complete'
    });
    this.subscriptions.delete(id);
  }

  private handleReconnect(): void {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

      console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);

      setTimeout(() => {
        this.connect().catch(console.error);
      }, delay);
    } else {
      console.error('Max reconnection attempts reached');
    }
  }

  disconnect(): void {
    if (this.ws) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }
  }
}

// Usage
const client = new IncidentStreamClient(
  'ws://localhost:8080/graphql/ws',
  'YOUR_JWT_TOKEN'
);

await client.connect();

// Subscribe to critical incidents
client.subscribe(
  'critical-incidents',
  `
    subscription {
      criticalIncidents {
        id
        title
        severity
        state
        createdAt
      }
    }
  `,
  {},
  (data) => {
    console.log('New critical incident:', data.criticalIncidents);
  }
);

// Subscribe to incident updates with variables
client.subscribe(
  'incident-updates',
  `
    subscription IncidentUpdates($severities: [Severity!]) {
      incidentUpdates(severities: $severities) {
        updateType
        incidentId
        timestamp
      }
    }
  `,
  { severities: ['P0', 'P1'] },
  (data) => {
    console.log('Incident update:', data.incidentUpdates);
  }
);

// Unsubscribe when done
// client.unsubscribe('critical-incidents');

// Disconnect
// client.disconnect();
```

---

### Node.js (ws library)

```typescript
import WebSocket from 'ws';
import { EventEmitter } from 'events';

interface SubscriptionOptions {
  query: string;
  variables?: Record<string, any>;
  operationName?: string;
}

class NodeIncidentClient extends EventEmitter {
  private ws: WebSocket | null = null;
  private url: string;
  private token: string;
  private subscriptions: Map<string, SubscriptionOptions> = new Map();

  constructor(url: string, token: string) {
    super();
    this.url = url;
    this.token = token;
  }

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url, {
        protocol: 'graphql-transport-ws',
        headers: {
          Authorization: `Bearer ${this.token}`
        }
      });

      this.ws.on('open', () => {
        this.send({
          type: 'connection_init',
          payload: {
            Authorization: `Bearer ${this.token}`
          }
        });
      });

      this.ws.on('message', (data: WebSocket.Data) => {
        const message = JSON.parse(data.toString());

        if (message.type === 'connection_ack') {
          this.emit('connected');
          resolve();
        } else if (message.type === 'next') {
          this.emit(`data:${message.id}`, message.payload.data);
        } else if (message.type === 'error') {
          this.emit(`error:${message.id}`, message.payload);
        } else if (message.type === 'complete') {
          this.emit(`complete:${message.id}`);
        }
      });

      this.ws.on('error', (error) => {
        this.emit('error', error);
        reject(error);
      });

      this.ws.on('close', () => {
        this.emit('disconnected');
      });
    });
  }

  subscribe(id: string, options: SubscriptionOptions): void {
    this.subscriptions.set(id, options);

    this.send({
      id,
      type: 'subscribe',
      payload: options
    });
  }

  private send(message: any): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  disconnect(): void {
    if (this.ws) {
      this.ws.close(1000);
      this.ws = null;
    }
  }
}

// Usage
const client = new NodeIncidentClient(
  'ws://localhost:8080/graphql/ws',
  process.env.AUTH_TOKEN!
);

await client.connect();

// Subscribe to incidents
client.subscribe('incidents', {
  query: `
    subscription {
      newIncidents(severities: [P0, P1]) {
        id
        title
        severity
        state
      }
    }
  `
});

// Handle events
client.on('data:incidents', (data) => {
  console.log('New incident:', data.newIncidents);

  // Process incident
  processIncident(data.newIncidents);
});

client.on('error:incidents', (error) => {
  console.error('Subscription error:', error);
});

client.on('complete:incidents', () => {
  console.log('Subscription completed');
});

// Graceful shutdown
process.on('SIGINT', () => {
  client.disconnect();
  process.exit(0);
});
```

---

### graphql-ws Client

The recommended client library for GraphQL WebSocket subscriptions.

```bash
npm install graphql-ws
```

```typescript
import { createClient, Client } from 'graphql-ws';
import WebSocket from 'ws';

const wsUrl = 'ws://localhost:8080/graphql/ws';
const authToken = 'YOUR_JWT_TOKEN';

const client: Client = createClient({
  url: wsUrl,
  webSocketImpl: WebSocket, // Use in Node.js; omit in browser
  connectionParams: {
    Authorization: `Bearer ${authToken}`
  },
  retryAttempts: 5,
  retryWait: (retries) => {
    return new Promise((resolve) => {
      setTimeout(resolve, Math.min(1000 * 2 ** retries, 30000));
    });
  },
  on: {
    connected: () => console.log('Connected'),
    closed: () => console.log('Disconnected'),
    error: (error) => console.error('Connection error:', error)
  }
});

// Subscribe to critical incidents
const unsubscribe = client.subscribe(
  {
    query: `
      subscription {
        criticalIncidents {
          id
          title
          severity
          state
          affectedResources
          createdAt
        }
      }
    `
  },
  {
    next: (data) => {
      console.log('Critical incident:', data.data.criticalIncidents);

      // Send notification
      sendPagerDutyAlert(data.data.criticalIncidents);
    },
    error: (error) => {
      console.error('Subscription error:', error);
    },
    complete: () => {
      console.log('Subscription completed');
    }
  }
);

// Subscribe with variables
const unsubscribeUpdates = client.subscribe(
  {
    query: `
      subscription IncidentUpdates($incidentIds: [UUID!]!) {
        incidentUpdates(incidentIds: $incidentIds) {
          updateType
          incidentId
          timestamp
        }
      }
    `,
    variables: {
      incidentIds: [
        '550e8400-e29b-41d4-a716-446655440000',
        '660e8400-e29b-41d4-a716-446655440001'
      ]
    }
  },
  {
    next: (data) => {
      console.log('Update:', data.data.incidentUpdates);
    },
    error: console.error,
    complete: () => console.log('Complete')
  }
);

// Cleanup
// unsubscribe();
// client.dispose();
```

---

### Apollo Client

For React applications using Apollo Client.

```bash
npm install @apollo/client graphql-ws
```

```typescript
import { ApolloClient, InMemoryCache, HttpLink, split } from '@apollo/client';
import { GraphQLWsLink } from '@apollo/client/link/subscriptions';
import { createClient } from 'graphql-ws';
import { getMainDefinition } from '@apollo/client/utilities';

const httpLink = new HttpLink({
  uri: 'http://localhost:8080/graphql',
  headers: {
    authorization: `Bearer ${localStorage.getItem('token')}`
  }
});

const wsLink = new GraphQLWsLink(
  createClient({
    url: 'ws://localhost:8080/graphql/ws',
    connectionParams: {
      Authorization: `Bearer ${localStorage.getItem('token')}`
    }
  })
);

// Split based on operation type
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

const client = new ApolloClient({
  link: splitLink,
  cache: new InMemoryCache()
});

export default client;
```

**React Component:**

```tsx
import React from 'react';
import { useSubscription, gql } from '@apollo/client';

const CRITICAL_INCIDENTS_SUBSCRIPTION = gql`
  subscription CriticalIncidents {
    criticalIncidents {
      id
      title
      severity
      state
      createdAt
      affectedResources
    }
  }
`;

const CriticalIncidentsMonitor: React.FC = () => {
  const { data, loading, error } = useSubscription(
    CRITICAL_INCIDENTS_SUBSCRIPTION
  );

  if (loading) return <div>Connecting...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <div className="critical-incidents">
      <h2>Critical Incidents</h2>
      {data?.criticalIncidents && (
        <div className="incident-alert">
          <h3>{data.criticalIncidents.title}</h3>
          <p>Severity: {data.criticalIncidents.severity}</p>
          <p>State: {data.criticalIncidents.state}</p>
          <p>Affected: {data.criticalIncidents.affectedResources.join(', ')}</p>
        </div>
      )}
    </div>
  );
};

export default CriticalIncidentsMonitor;
```

---

## Python

### websockets Library

```bash
pip install websockets
```

```python
import asyncio
import json
import logging
from typing import Dict, Callable, Any
from websockets.client import connect, WebSocketClientProtocol

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class IncidentStreamClient:
    def __init__(self, url: str, token: str):
        self.url = url
        self.token = token
        self.ws: WebSocketClientProtocol = None
        self.subscriptions: Dict[str, Callable] = {}
        self.running = False

    async def connect(self):
        """Establish WebSocket connection"""
        self.ws = await connect(
            self.url,
            subprotocols=['graphql-transport-ws'],
            extra_headers={
                'Authorization': f'Bearer {self.token}'
            }
        )

        # Initialize connection
        await self.ws.send(json.dumps({
            'type': 'connection_init',
            'payload': {
                'Authorization': f'Bearer {self.token}'
            }
        }))

        # Wait for acknowledgment
        response = await self.ws.recv()
        message = json.loads(response)

        if message['type'] != 'connection_ack':
            raise ConnectionError('Connection not acknowledged')

        logger.info('WebSocket connected')
        self.running = True

        # Start message handler
        asyncio.create_task(self._handle_messages())

    async def _handle_messages(self):
        """Handle incoming messages"""
        try:
            async for message_str in self.ws:
                message = json.loads(message_str)
                await self._process_message(message)
        except Exception as e:
            logger.error(f'Message handling error: {e}')
        finally:
            self.running = False

    async def _process_message(self, message: Dict[str, Any]):
        """Process individual message"""
        msg_type = message.get('type')
        msg_id = message.get('id')

        if msg_type == 'next' and msg_id in self.subscriptions:
            handler = self.subscriptions[msg_id]
            data = message.get('payload', {}).get('data')
            await handler(data)

        elif msg_type == 'error':
            logger.error(f'Subscription error: {message.get("payload")}')

        elif msg_type == 'complete':
            logger.info(f'Subscription {msg_id} completed')
            if msg_id in self.subscriptions:
                del self.subscriptions[msg_id]

    async def subscribe(
        self,
        subscription_id: str,
        query: str,
        handler: Callable,
        variables: Dict[str, Any] = None
    ):
        """Subscribe to a GraphQL subscription"""
        self.subscriptions[subscription_id] = handler

        await self.ws.send(json.dumps({
            'id': subscription_id,
            'type': 'subscribe',
            'payload': {
                'query': query,
                'variables': variables or {}
            }
        }))

        logger.info(f'Subscribed: {subscription_id}')

    async def unsubscribe(self, subscription_id: str):
        """Unsubscribe from a subscription"""
        await self.ws.send(json.dumps({
            'id': subscription_id,
            'type': 'complete'
        }))

        if subscription_id in self.subscriptions:
            del self.subscriptions[subscription_id]

        logger.info(f'Unsubscribed: {subscription_id}')

    async def disconnect(self):
        """Close WebSocket connection"""
        self.running = False
        if self.ws:
            await self.ws.close()
        logger.info('WebSocket disconnected')


# Usage
async def handle_critical_incident(data):
    """Handle critical incident notification"""
    incident = data['criticalIncidents']
    print(f'CRITICAL: {incident["title"]} (Severity: {incident["severity"]})')

    # Send notification
    # await send_slack_alert(incident)

async def handle_incident_update(data):
    """Handle incident update"""
    update = data['incidentUpdates']
    print(f'Update: {update["updateType"]} - {update["incidentId"]}')

async def main():
    client = IncidentStreamClient(
        'ws://localhost:8080/graphql/ws',
        'YOUR_JWT_TOKEN'
    )

    try:
        await client.connect()

        # Subscribe to critical incidents
        await client.subscribe(
            'critical-incidents',
            '''
            subscription {
              criticalIncidents {
                id
                title
                severity
                state
                affectedResources
                createdAt
              }
            }
            ''',
            handle_critical_incident
        )

        # Subscribe to incident updates
        await client.subscribe(
            'incident-updates',
            '''
            subscription IncidentUpdates($severities: [Severity!]) {
              incidentUpdates(severities: $severities) {
                updateType
                incidentId
                timestamp
              }
            }
            ''',
            handle_incident_update,
            {'severities': ['P0', 'P1']}
        )

        # Keep running
        while client.running:
            await asyncio.sleep(1)

    except KeyboardInterrupt:
        logger.info('Shutting down...')
    finally:
        await client.disconnect()

if __name__ == '__main__':
    asyncio.run(main())
```

---

### gql Client

```bash
pip install gql[websockets]
```

```python
from gql import Client, gql
from gql.transport.websockets import WebsocketsTransport
import asyncio

# Configure transport
transport = WebsocketsTransport(
    url='ws://localhost:8080/graphql/ws',
    init_payload={
        'Authorization': 'Bearer YOUR_JWT_TOKEN'
    },
    subprotocols=[WebsocketsTransport.GRAPHQLWS_SUBPROTOCOL]
)

# Create client
client = Client(
    transport=transport,
    fetch_schema_from_transport=False
)

# Define subscription
subscription = gql('''
    subscription {
      criticalIncidents {
        id
        title
        severity
        state
        createdAt
      }
    }
''')

async def main():
    async with client as session:
        async for result in session.subscribe(subscription):
            incident = result['criticalIncidents']
            print(f'Critical incident: {incident["title"]}')
            print(f'Severity: {incident["severity"]}')
            print(f'State: {incident["state"]}')
            print('---')

if __name__ == '__main__':
    asyncio.run(main())
```

---

## Rust

### tokio-tungstenite

```toml
[dependencies]
tokio = { version = "1.35", features = ["full"] }
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
futures-util = "0.3"
```

```rust
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    #[serde(rename = "connection_init")]
    ConnectionInit { payload: serde_json::Value },

    #[serde(rename = "connection_ack")]
    ConnectionAck,

    #[serde(rename = "subscribe")]
    Subscribe {
        id: String,
        payload: SubscriptionPayload,
    },

    #[serde(rename = "next")]
    Next {
        id: String,
        payload: serde_json::Value,
    },

    #[serde(rename = "error")]
    Error {
        id: String,
        payload: Vec<serde_json::Value>,
    },

    #[serde(rename = "complete")]
    Complete { id: String },
}

#[derive(Debug, Serialize, Deserialize)]
struct SubscriptionPayload {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<serde_json::Value>,
}

type WsConnection = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct IncidentStreamClient {
    ws: WsConnection,
}

impl IncidentStreamClient {
    async fn connect(url: &str, token: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (ws, _) = connect_async(url).await?;
        let mut client = Self { ws };

        // Initialize connection
        client.send_message(WsMessage::ConnectionInit {
            payload: json!({
                "Authorization": format!("Bearer {}", token)
            }),
        }).await?;

        // Wait for acknowledgment
        if let Some(msg) = client.receive_message().await? {
            match msg {
                WsMessage::ConnectionAck => {
                    println!("Connection established");
                }
                _ => return Err("Expected connection_ack".into()),
            }
        }

        Ok(client)
    }

    async fn send_message(&mut self, msg: WsMessage) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(&msg)?;
        self.ws.send(Message::Text(json)).await?;
        Ok(())
    }

    async fn receive_message(&mut self) -> Result<Option<WsMessage>, Box<dyn std::error::Error>> {
        if let Some(msg) = self.ws.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                let ws_msg: WsMessage = serde_json::from_str(&text)?;
                return Ok(Some(ws_msg));
            }
        }
        Ok(None)
    }

    async fn subscribe(
        &mut self,
        id: String,
        query: String,
        variables: Option<serde_json::Value>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.send_message(WsMessage::Subscribe {
            id,
            payload: SubscriptionPayload { query, variables },
        }).await
    }

    async fn handle_messages(
        mut self,
        mut handler: impl FnMut(String, serde_json::Value),
    ) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(msg) = self.receive_message().await? {
            match msg {
                WsMessage::Next { id, payload } => {
                    handler(id, payload);
                }
                WsMessage::Error { id, payload } => {
                    eprintln!("Subscription error ({}): {:?}", id, payload);
                }
                WsMessage::Complete { id } => {
                    println!("Subscription completed: {}", id);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = IncidentStreamClient::connect(
        "ws://localhost:8080/graphql/ws",
        "YOUR_JWT_TOKEN",
    ).await?;

    // Subscribe to critical incidents
    client.subscribe(
        "critical-incidents".to_string(),
        r#"
        subscription {
          criticalIncidents {
            id
            title
            severity
            state
            createdAt
          }
        }
        "#.to_string(),
        None,
    ).await?;

    // Handle incoming messages
    client.handle_messages(|id, data| {
        println!("Received data for {}: {:?}", id, data);
    }).await?;

    Ok(())
}
```

---

## Go

### gorilla/websocket

```go
package main

import (
    "encoding/json"
    "fmt"
    "log"
    "net/http"
    "time"

    "github.com/gorilla/websocket"
)

type WsMessage struct {
    Type    string          `json:"type"`
    ID      string          `json:"id,omitempty"`
    Payload json.RawMessage `json:"payload,omitempty"`
}

type SubscriptionPayload struct {
    Query     string                 `json:"query"`
    Variables map[string]interface{} `json:"variables,omitempty"`
}

type IncidentStreamClient struct {
    conn          *websocket.Conn
    token         string
    subscriptions map[string]func(json.RawMessage)
}

func NewIncidentStreamClient(url, token string) (*IncidentStreamClient, error) {
    headers := http.Header{}
    headers.Set("Authorization", fmt.Sprintf("Bearer %s", token))
    headers.Set("Sec-WebSocket-Protocol", "graphql-transport-ws")

    conn, _, err := websocket.DefaultDialer.Dial(url, headers)
    if err != nil {
        return nil, err
    }

    client := &IncidentStreamClient{
        conn:          conn,
        token:         token,
        subscriptions: make(map[string]func(json.RawMessage)),
    }

    // Initialize connection
    initPayload := map[string]interface{}{
        "Authorization": fmt.Sprintf("Bearer %s", token),
    }

    if err := client.sendMessage("connection_init", "", initPayload); err != nil {
        return nil, err
    }

    // Wait for acknowledgment
    var msg WsMessage
    if err := conn.ReadJSON(&msg); err != nil {
        return nil, err
    }

    if msg.Type != "connection_ack" {
        return nil, fmt.Errorf("expected connection_ack, got %s", msg.Type)
    }

    log.Println("WebSocket connected")

    // Start message handler
    go client.handleMessages()

    return client, nil
}

func (c *IncidentStreamClient) sendMessage(msgType, id string, payload interface{}) error {
    msg := WsMessage{
        Type: msgType,
        ID:   id,
    }

    if payload != nil {
        payloadBytes, err := json.Marshal(payload)
        if err != nil {
            return err
        }
        msg.Payload = payloadBytes
    }

    return c.conn.WriteJSON(msg)
}

func (c *IncidentStreamClient) Subscribe(
    id, query string,
    variables map[string]interface{},
    handler func(json.RawMessage),
) error {
    c.subscriptions[id] = handler

    payload := SubscriptionPayload{
        Query:     query,
        Variables: variables,
    }

    return c.sendMessage("subscribe", id, payload)
}

func (c *IncidentStreamClient) Unsubscribe(id string) error {
    delete(c.subscriptions, id)
    return c.sendMessage("complete", id, nil)
}

func (c *IncidentStreamClient) handleMessages() {
    for {
        var msg WsMessage
        if err := c.conn.ReadJSON(&msg); err != nil {
            log.Printf("Read error: %v", err)
            break
        }

        switch msg.Type {
        case "next":
            if handler, ok := c.subscriptions[msg.ID]; ok {
                handler(msg.Payload)
            }

        case "error":
            log.Printf("Subscription error (%s): %s", msg.ID, string(msg.Payload))

        case "complete":
            log.Printf("Subscription completed: %s", msg.ID)
            delete(c.subscriptions, msg.ID)

        case "pong":
            // Handle pong
        }
    }
}

func (c *IncidentStreamClient) Close() error {
    return c.conn.Close()
}

func main() {
    client, err := NewIncidentStreamClient(
        "ws://localhost:8080/graphql/ws",
        "YOUR_JWT_TOKEN",
    )
    if err != nil {
        log.Fatal(err)
    }
    defer client.Close()

    // Subscribe to critical incidents
    err = client.Subscribe(
        "critical-incidents",
        `subscription {
          criticalIncidents {
            id
            title
            severity
            state
            createdAt
          }
        }`,
        nil,
        func(data json.RawMessage) {
            fmt.Printf("Received: %s\n", string(data))
        },
    )
    if err != nil {
        log.Fatal(err)
    }

    // Keep alive
    time.Sleep(60 * time.Second)
}
```

---

## Common Patterns

### Authentication

**Token Refresh Pattern:**

```typescript
class AuthenticatedClient {
  private token: string;
  private refreshToken: string;
  private tokenExpiry: Date;

  async ensureValidToken(): Promise<string> {
    if (this.isTokenExpired()) {
      await this.refreshAuthToken();
    }
    return this.token;
  }

  private isTokenExpired(): boolean {
    return new Date() >= this.tokenExpiry;
  }

  private async refreshAuthToken(): Promise<void> {
    const response = await fetch('/auth/refresh', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refreshToken: this.refreshToken })
    });

    const data = await response.json();
    this.token = data.token;
    this.tokenExpiry = new Date(data.expiresAt);
  }

  async connect(): Promise<void> {
    const token = await this.ensureValidToken();
    // Use token to connect...
  }
}
```

---

### Reconnection Strategy

**Exponential Backoff with Jitter:**

```typescript
class ReconnectionManager {
  private attempt = 0;
  private maxAttempts = 10;
  private baseDelay = 1000;
  private maxDelay = 30000;

  async reconnect(connectFn: () => Promise<void>): Promise<void> {
    while (this.attempt < this.maxAttempts) {
      try {
        await connectFn();
        this.attempt = 0; // Reset on success
        return;
      } catch (error) {
        this.attempt++;
        const delay = this.calculateDelay();

        console.log(
          `Reconnection attempt ${this.attempt} failed. ` +
          `Retrying in ${delay}ms...`
        );

        await this.sleep(delay);
      }
    }

    throw new Error('Max reconnection attempts exceeded');
  }

  private calculateDelay(): number {
    // Exponential backoff: baseDelay * 2^attempt
    const exponential = this.baseDelay * Math.pow(2, this.attempt);

    // Cap at maxDelay
    const capped = Math.min(exponential, this.maxDelay);

    // Add jitter (±25%)
    const jitter = capped * 0.25 * (Math.random() - 0.5);

    return Math.floor(capped + jitter);
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  reset(): void {
    this.attempt = 0;
  }
}

// Usage
const reconnectionManager = new ReconnectionManager();

try {
  await reconnectionManager.reconnect(async () => {
    await client.connect();
  });
} catch (error) {
  console.error('Failed to reconnect:', error);
  // Alert operations team
}
```

---

### Error Handling

**Comprehensive Error Handler:**

```typescript
interface ErrorContext {
  subscriptionId?: string;
  operation?: string;
  timestamp: Date;
}

class ErrorHandler {
  handleConnectionError(error: Error, context: ErrorContext): void {
    console.error('Connection error:', error, context);

    // Log to monitoring service
    this.logToMonitoring('connection_error', {
      error: error.message,
      ...context
    });

    // Trigger reconnection
    this.triggerReconnection();
  }

  handleSubscriptionError(
    error: any,
    context: ErrorContext
  ): void {
    const errorCode = error.extensions?.code;

    switch (errorCode) {
      case 'UNAUTHENTICATED':
      case 'UNAUTHORIZED':
        // Refresh token and resubscribe
        this.refreshTokenAndResubscribe(context.subscriptionId);
        break;

      case 'RATE_LIMIT_EXCEEDED':
        // Back off and retry
        this.backoffAndRetry(context.subscriptionId);
        break;

      case 'INTERNAL_SERVER_ERROR':
        // Log and alert
        this.alertOperations(error, context);
        break;

      default:
        console.error('Unknown subscription error:', error);
    }
  }

  handleMessageError(error: Error, rawMessage: string): void {
    console.error('Message parsing error:', error);
    console.error('Raw message:', rawMessage);

    // Log malformed message for debugging
    this.logToMonitoring('message_parse_error', {
      error: error.message,
      rawMessage: rawMessage.substring(0, 100)
    });
  }

  private logToMonitoring(event: string, data: any): void {
    // Send to monitoring service (e.g., Datadog, Sentry)
    fetch('/api/monitoring/log', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ event, data, timestamp: new Date() })
    });
  }

  private triggerReconnection(): void {
    // Implementation depends on client architecture
  }

  private refreshTokenAndResubscribe(subscriptionId?: string): void {
    // Implementation depends on auth system
  }

  private backoffAndRetry(subscriptionId?: string): void {
    // Implementation depends on retry strategy
  }

  private alertOperations(error: any, context: ErrorContext): void {
    // Send to PagerDuty, Slack, etc.
  }
}
```

---

### State Management

**React State Management with Subscriptions:**

```typescript
import { useEffect, useState, useCallback } from 'react';
import { createClient } from 'graphql-ws';

interface Incident {
  id: string;
  title: string;
  severity: string;
  state: string;
  createdAt: string;
}

function useIncidentSubscription(severities: string[]) {
  const [incidents, setIncidents] = useState<Incident[]>([]);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const addIncident = useCallback((incident: Incident) => {
    setIncidents(prev => [incident, ...prev].slice(0, 100)); // Keep last 100
  }, []);

  useEffect(() => {
    const client = createClient({
      url: 'ws://localhost:8080/graphql/ws',
      connectionParams: {
        Authorization: `Bearer ${localStorage.getItem('token')}`
      }
    });

    const unsubscribe = client.subscribe(
      {
        query: `
          subscription NewIncidents($severities: [Severity!]) {
            newIncidents(severities: $severities) {
              id
              title
              severity
              state
              createdAt
            }
          }
        `,
        variables: { severities }
      },
      {
        next: (data) => {
          setConnected(true);
          addIncident(data.data.newIncidents);
        },
        error: (err) => {
          setConnected(false);
          setError(err);
        },
        complete: () => {
          setConnected(false);
        }
      }
    );

    return () => {
      unsubscribe();
    };
  }, [severities, addIncident]);

  return { incidents, connected, error };
}

// Component usage
function IncidentDashboard() {
  const { incidents, connected, error } = useIncidentSubscription(['P0', 'P1']);

  if (error) {
    return <div>Error: {error.message}</div>;
  }

  return (
    <div>
      <div className="status">
        {connected ? '● Connected' : '○ Disconnected'}
      </div>
      <div className="incidents">
        {incidents.map(incident => (
          <div key={incident.id} className="incident-card">
            <h3>{incident.title}</h3>
            <span className={`severity ${incident.severity}`}>
              {incident.severity}
            </span>
            <span className="state">{incident.state}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
```

---

## Next Steps

1. Review [WebSocket API Reference](./WEBSOCKET_API_REFERENCE.md) for complete API details
2. Check [WebSocket Deployment Guide](./WEBSOCKET_DEPLOYMENT_GUIDE.md) for production setup
3. Explore [GraphQL Examples](./GRAPHQL_EXAMPLES.md) for more query patterns

## Related Documentation

- [WebSocket Streaming Guide](./WEBSOCKET_STREAMING_GUIDE.md) - Architecture overview
- [GraphQL Integration Guide](./GRAPHQL_INTEGRATION_GUIDE.md) - General GraphQL integration
- [Integration Guide](./integration-guide.md) - Complete integration patterns
