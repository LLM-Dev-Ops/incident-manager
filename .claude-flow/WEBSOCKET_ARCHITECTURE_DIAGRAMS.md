# WebSocket Architecture - Visual Diagrams

**Version**: 1.0
**Date**: 2025-11-12

---

## 1. System Component Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          LLM INCIDENT MANAGER                                │
│                     WebSocket Streaming Architecture                         │
└─────────────────────────────────────────────────────────────────────────────┘

                                 CLIENTS
         ┌──────────────┬──────────────┬──────────────┬──────────────┐
         │              │              │              │              │
         ▼              ▼              ▼              ▼              ▼
    ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐
    │  Web   │    │ Mobile │    │  CLI   │    │Monitor │    │ Custom │
    │  UI    │    │  App   │    │  Tool  │    │ System │    │ Client │
    └───┬────┘    └───┬────┘    └───┬────┘    └───┬────┘    └───┬────┘
        │             │             │             │             │
        └─────────────┴─────────────┴─────────────┴─────────────┘
                                    │
                           WSS:// or WS://
                                    │
        ┌───────────────────────────▼───────────────────────────┐
        │            LOAD BALANCER / REVERSE PROXY              │
        │           (Nginx, HAProxy, K8s Ingress)               │
        │  • TLS Termination   • Sticky Sessions   • Routing   │
        └───────────────────────────┬───────────────────────────┘
                                    │
              ┌─────────────────────┼─────────────────────┐
              │                     │                     │
              ▼                     ▼                     ▼
    ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
    │   Instance 1     │  │   Instance 2     │  │   Instance 3     │
    │ ┌──────────────┐ │  │ ┌──────────────┐ │  │ ┌──────────────┐ │
    │ │ WebSocket    │ │  │ │ WebSocket    │ │  │ │ WebSocket    │ │
    │ │ Handler      │ │  │ │ Handler      │ │  │ │ Handler      │ │
    │ └──────┬───────┘ │  │ └──────┬───────┘ │  │ └──────┬───────┘ │
    │        │         │  │        │         │  │        │         │
    │ ┌──────▼───────┐ │  │ ┌──────▼───────┐ │  │ ┌──────▼───────┐ │
    │ │ Connection   │ │  │ │ Connection   │ │  │ │ Connection   │ │
    │ │ Manager      │ │  │ │ Manager      │ │  │ │ Manager      │ │
    │ │ • Sessions   │ │  │ │ • Sessions   │ │  │ │ • Sessions   │ │
    │ │ • Filtering  │ │  │ │ • Filtering  │ │  │ │ • Filtering  │ │
    │ └──────┬───────┘ │  │ └──────┬───────┘ │  │ └──────┬───────┘ │
    │        │         │  │        │         │  │        │         │
    │ ┌──────▼───────┐ │  │ ┌──────▼───────┐ │  │ ┌──────▼───────┐ │
    │ │ Event        │ │  │ │ Event        │ │  │ │ Event        │ │
    │ │ Broadcaster  │ │  │ │ Broadcaster  │ │  │ │ Broadcaster  │ │
    │ └──────┬───────┘ │  │ └──────┬───────┘ │  │ └──────┬───────┘ │
    │        │         │  │        │         │  │        │         │
    └────────┼─────────┘  └────────┼─────────┘  └────────┼─────────┘
             │                     │                     │
             └─────────────────────┼─────────────────────┘
                                   │
                        ┌──────────▼──────────┐
                        │   Redis Pub/Sub     │
                        │  (Multi-Instance)   │
                        │  • Event Routing    │
                        │  • State Sync       │
                        └──────────┬──────────┘
                                   │
             ┌─────────────────────┼─────────────────────┐
             │                     │                     │
             ▼                     ▼                     ▼
    ┌────────────────┐   ┌────────────────┐   ┌────────────────┐
    │   Incident     │   │  Escalation    │   │   Playbook     │
    │   Processor    │   │    Engine      │   │   Service      │
    │                │   │                │   │                │
    │ Publishes:     │   │ Publishes:     │   │ Publishes:     │
    │ • Incidents    │   │ • Escalations  │   │ • Playbooks    │
    │ • Alerts       │   │                │   │ • Actions      │
    └────────────────┘   └────────────────┘   └────────────────┘
```

---

## 2. WebSocket Connection Lifecycle

```
┌──────────┐                                              ┌──────────┐
│  Client  │                                              │  Server  │
└─────┬────┘                                              └─────┬────┘
      │                                                         │
      │  1. HTTP Upgrade Request                               │
      │  GET /ws HTTP/1.1                                      │
      │  Upgrade: websocket                                    │
      │  Connection: Upgrade                                   │
      │───────────────────────────────────────────────────────►│
      │                                                         │
      │                         2. Validate & Accept           │
      │                            Create Session              │
      │                            Register Connection         │
      │                                                         │
      │  3. HTTP 101 Switching Protocols                       │
      │◄───────────────────────────────────────────────────────│
      │                                                         │
      │  4. Welcome Message                                    │
      │  { session_id, server_time }                           │
      │◄───────────────────────────────────────────────────────│
      │                                                         │
      │  5. Subscribe Message                                  │
      │  { subscription_id, filters }                          │
      │───────────────────────────────────────────────────────►│
      │                                                         │
      │                         6. Register Subscription       │
      │                            Apply Filters               │
      │                                                         │
      │  7. Subscribed Confirmation                            │
      │  { subscription_id, filters }                          │
      │◄───────────────────────────────────────────────────────│
      │                                                         │
      │                                                         │
      │ ╔════════════════════════════════════════════════════╗ │
      │ ║          ACTIVE CONNECTION PHASE                   ║ │
      │ ╚════════════════════════════════════════════════════╝ │
      │                                                         │
      │  8. Event Stream (continuous)                          │
      │  { event_type, data, ... }                             │
      │◄───────────────────────────────────────────────────────│
      │  { event_type, data, ... }                             │
      │◄───────────────────────────────────────────────────────│
      │  { event_type, data, ... }                             │
      │◄───────────────────────────────────────────────────────│
      │                                                         │
      │  9. Heartbeat (every 30s)                              │
      │  Ping                                                  │
      │───────────────────────────────────────────────────────►│
      │  Pong                                                  │
      │◄───────────────────────────────────────────────────────│
      │                                                         │
      │  10. Additional Subscriptions (optional)               │
      │  Subscribe                                             │
      │───────────────────────────────────────────────────────►│
      │  Subscribed                                            │
      │◄───────────────────────────────────────────────────────│
      │                                                         │
      │  11. Unsubscribe (optional)                            │
      │  Unsubscribe { subscription_id }                       │
      │───────────────────────────────────────────────────────►│
      │  Unsubscribed                                          │
      │◄───────────────────────────────────────────────────────│
      │                                                         │
      │                                                         │
      │ ╔════════════════════════════════════════════════════╗ │
      │ ║          DISCONNECTION PHASE                       ║ │
      │ ╚════════════════════════════════════════════════════╝ │
      │                                                         │
      │  12. Close Connection                                  │
      │  Close Frame                                           │
      │───────────────────────────────────────────────────────►│
      │                                                         │
      │                         13. Cleanup                    │
      │                             Unregister Connection      │
      │                             Remove Subscriptions       │
      │                             Stop Heartbeat             │
      │                                                         │
      │  14. Close Acknowledgment                              │
      │  Close Frame                                           │
      │◄───────────────────────────────────────────────────────│
      │                                                         │
     ╳                                                         ╳
 Connection Closed                                    Connection Closed
```

---

## 3. Event Broadcasting Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                     EVENT PUBLISHING FLOW                            │
└─────────────────────────────────────────────────────────────────────┘

    ┌──────────────┐
    │  Service     │  (e.g., IncidentProcessor)
    │  Code        │
    └──────┬───────┘
           │
           │ 1. Create Event
           │    incident_created(incident)
           ▼
    ┌──────────────┐
    │ Event        │
    │ Handlers     │  (handlers.rs)
    └──────┬───────┘
           │
           │ 2. Call Broadcaster
           │    publish(event)
           ▼
    ┌──────────────────────────────────────────┐
    │     Event Broadcaster                     │
    │     (broadcaster.rs)                      │
    │                                           │
    │  ┌─────────────────────────────────────┐ │
    │  │ 1. Create EventEnvelope             │ │
    │  │    • Generate message_id (UUID)     │ │
    │  │    • Add timestamp                  │ │
    │  │    • Determine priority             │ │
    │  │    • Record metrics                 │ │
    │  └─────────────────────────────────────┘ │
    │                                           │
    │  ┌─────────────────────────────────────┐ │
    │  │ 2. Broadcast via Channel            │ │
    │  │    tokio::broadcast::send()         │ │
    │  └──────────────┬──────────────────────┘ │
    └─────────────────┼────────────────────────┘
                      │
         ┌────────────┼────────────┐
         │            │            │
         ▼            ▼            ▼
    ┌────────┐  ┌────────┐  ┌────────┐
    │Internal│  │Internal│  │ WS     │
    │Receiver│  │Receiver│  │Conns   │
    └────────┘  └────────┘  └───┬────┘
                                 │
                                 │ 3. For each connection
                                 ▼
                    ┌─────────────────────────────┐
                    │ Connection Manager          │
                    │ broadcast_event()           │
                    │                             │
                    │ For each active connection: │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    │              │              │
                    ▼              ▼              ▼
              ┌──────────┐   ┌──────────┐   ┌──────────┐
              │Connection│   │Connection│   │Connection│
              │    1     │   │    2     │   │    3     │
              └────┬─────┘   └────┬─────┘   └────┬─────┘
                   │              │              │
                   │ 4. Check if should receive  │
                   │    • Get session             │
                   │    • Check subscriptions     │
                   │    • Apply filters           │
                   ▼              ▼              ▼
              ┌─────────┐    ┌─────────┐    ┌─────────┐
              │ Filter  │    │ Filter  │    │ Filter  │
              │ Match?  │    │ Match?  │    │ Match?  │
              └────┬────┘    └────┬────┘    └────┬────┘
                   │              │              │
                   │ YES          │ NO           │ YES
                   ▼              ╳              ▼
              ┌─────────┐                    ┌─────────┐
              │ Send to │                    │ Send to │
              │ Message │                    │ Message │
              │ Queue   │                    │ Queue   │
              └────┬────┘                    └────┬────┘
                   │                              │
                   │ 5. Async send                │
                   │    mpsc::send()              │
                   ▼                              ▼
              ┌──────────┐                   ┌──────────┐
              │ Writer   │                   │ Writer   │
              │ Task     │                   │ Task     │
              └────┬─────┘                   └────┬─────┘
                   │                              │
                   │ 6. Serialize & send          │
                   │    JSON encode               │
                   │    WebSocket send            │
                   ▼                              ▼
              ┌──────────┐                   ┌──────────┐
              │ Client 1 │                   │ Client 3 │
              └──────────┘                   └──────────┘


    ┌─────────────────────────────────────────────────────────┐
    │  METRICS RECORDED:                                      │
    │  • websocket_events_broadcast_total (increment)         │
    │  • websocket_events_delivered_total (per client)        │
    │  • websocket_message_latency_seconds (histogram)        │
    │  • websocket_send_errors_total (on failure)             │
    └─────────────────────────────────────────────────────────┘
```

---

## 4. Event Filtering Decision Tree

```
                          ┌──────────────────┐
                          │  Event Received  │
                          └────────┬─────────┘
                                   │
                                   ▼
                    ┌──────────────────────────────┐
                    │  For Each Active Session     │
                    │  (ConnectionManager)         │
                    └──────────────┬───────────────┘
                                   │
                                   ▼
                    ┌──────────────────────────────┐
                    │  Does session have any       │
                    │  subscriptions?              │
                    └──────┬──────────────┬────────┘
                           │ NO           │ YES
                           ▼              ▼
                          ╳ Skip    ┌─────────────────┐
                                    │  Get interested │
                                    │  event types    │
                                    └────────┬────────┘
                                             │
                                             ▼
                          ┌──────────────────────────────────┐
                          │  Is event type in interested     │
                          │  set?                            │
                          └────┬──────────────────┬──────────┘
                               │ NO               │ YES
                               ▼                  ▼
                              ╳ Skip      ┌───────────────────┐
                                          │ For each          │
                                          │ subscription      │
                                          └────────┬──────────┘
                                                   │
                                                   ▼
                          ┌────────────────────────────────────────────┐
                          │  Check Event Type Filter                   │
                          │  (empty = match all)                       │
                          └──────────────┬──────────────┬──────────────┘
                                         │ NO           │ YES
                                         ▼              ▼
                                        ╳ Skip   ┌──────────────────┐
                                                 │ Check Incident?  │
                                                 │ or Alert?        │
                                                 └───┬──────────┬───┘
                                                     │          │
                                      ┌──────────────┘          └──────────────┐
                                      │ INCIDENT                ALERT           │
                                      ▼                         ▼               │
                    ┌─────────────────────────────┐  ┌─────────────────────────┴──┐
                    │ INCIDENT FILTER CHAIN       │  │ ALERT FILTER CHAIN         │
                    └──────────────┬──────────────┘  └──────────────┬─────────────┘
                                   │                                 │
          ┌────────────────────────┼─────────────────────────────────┘
          │                        │
          ▼                        ▼
    ┌──────────────┐         ┌──────────────┐
    │ Check        │         │ Check        │
    │ Severity     │         │ Severity     │
    │ Filter       │         │ Filter       │
    └──┬───────────┘         └──┬───────────┘
       │ Match                  │ Match
       ▼                        ▼
    ┌──────────────┐         ┌──────────────┐
    │ Check        │         │ Check        │
    │ State        │         │ Source       │
    │ Filter       │         │ Filter       │
    └──┬───────────┘         └──┬───────────┘
       │ Match                  │ Match
       ▼                        ▼
    ┌──────────────┐         ┌──────────────┐
    │ Check        │         │ Check        │
    │ Source       │         │ Resources    │
    │ Filter       │         │ Filter       │
    └──┬───────────┘         └──┬───────────┘
       │ Match                  │ Match
       ▼                        ▼
    ┌──────────────┐         ┌──────────────┐
    │ Check        │         │ Check        │
    │ Resources    │         │ Labels       │
    │ Filter       │         │ Filter       │
    └──┬───────────┘         └──┬───────────┘
       │ Match                  │ Match
       ▼                        │
    ┌──────────────┐            │
    │ Check        │            │
    │ Labels       │            │
    │ Filter       │            │
    └──┬───────────┘            │
       │ Match                  │
       ▼                        │
    ┌──────────────┐            │
    │ Check        │            │
    │ Incident IDs │            │
    │ Filter       │            │
    └──┬───────────┘            │
       │ Match                  │
       │                        │
       └────────┬───────────────┘
                │
                ▼
    ┌───────────────────────────┐
    │  ALL FILTERS PASSED       │
    │  Send event to connection │
    └───────────────────────────┘


    FILTER SEMANTICS:
    ┌────────────────────────────────────────────┐
    │ • Empty array = MATCH ALL                  │
    │ • Multiple values in array = OR (any)      │
    │ • Multiple filter types = AND (all)        │
    │ • Labels = AND (must match all key-value)  │
    └────────────────────────────────────────────┘
```

---

## 5. Session State Machine

```
                          ┌─────────────┐
                          │  START      │
                          └──────┬──────┘
                                 │
                                 │ Connection
                                 │ Established
                                 ▼
                    ┌────────────────────────┐
                    │                        │
                    │  CONNECTED             │
                    │                        │
                    │  • Send Welcome        │
                    │  • Create Session      │
                    │  • Start Heartbeat     │
                    │                        │
                    └────────┬───────────────┘
                             │
                             │ Subscribe
                             │ Message
                             ▼
                    ┌────────────────────────┐
                    │                        │
                    │  SUBSCRIBED            │
                    │                        │
                    │  • Add Subscription    │
                    │  • Apply Filters       │
                    │  • Start Streaming     │
                    │                        │
                    └───┬────────────────┬───┘
                        │                │
           ┌────────────┘                └────────────┐
           │                                          │
           │ More Subscriptions                Unsubscribe
           │                                          │
           ▼                                          ▼
    ┌──────────────┐                      ┌──────────────────┐
    │  ACTIVE      │◄─────────────────────│  UNSUBSCRIBING   │
    │              │                      │                  │
    │  • Multiple  │                      │  • Remove Sub    │
    │    Subs      │                      │  • Check Count   │
    │  • Streaming │                      │                  │
    │  • Heartbeat │                      └──────────────────┘
    └──────┬───────┘
           │
           │ Activity
           │ (any message)
           │
           ├───────────────────────────────────┐
           │                                   │
           │ Timeout (no activity)             │ Close
           ▼                                   │ Message
    ┌──────────────┐                          │
    │  IDLE        │                          │
    │              │                          │
    │  • No msgs   │                          │
    │  • Timeout   │                          │
    │    Check     │                          │
    └──────┬───────┘                          │
           │                                   │
           │ Cleanup                           │
           │ Interval                          │
           │                                   │
           └───────────┬───────────────────────┘
                       │
                       ▼
            ┌──────────────────────┐
            │                      │
            │  DISCONNECTING       │
            │                      │
            │  • Stop Heartbeat    │
            │  • Remove Subs       │
            │  • Unregister        │
            │  • Send Close        │
            │                      │
            └──────────┬───────────┘
                       │
                       ▼
            ┌──────────────────────┐
            │                      │
            │  CLOSED              │
            │                      │
            │  • Cleanup Complete  │
            │  • Metrics Recorded  │
            │                      │
            └──────────────────────┘


    TIMEOUTS:
    ┌────────────────────────────────────────┐
    │ Heartbeat Interval:  30 seconds        │
    │ Session Timeout:     300 seconds (5m)  │
    │ Cleanup Interval:    60 seconds        │
    └────────────────────────────────────────┘
```

---

## 6. Message Queue Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    PER-CONNECTION MESSAGE FLOW                       │
└─────────────────────────────────────────────────────────────────────┘

                    Event Broadcaster
                           │
                           │ broadcast_event()
                           ▼
                    ┌──────────────┐
                    │ Connection   │
                    │ Manager      │
                    └──────┬───────┘
                           │
                           │ For each matching connection
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
        ▼                  ▼                  ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│ Connection 1  │  │ Connection 2  │  │ Connection 3  │
└───────┬───────┘  └───────┬───────┘  └───────┬───────┘
        │                  │                  │
        │ connection.send(message)            │
        │                  │                  │
        ▼                  ▼                  ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│ Unbounded     │  │ Unbounded     │  │ Unbounded     │
│ MPSC Channel  │  │ MPSC Channel  │  │ MPSC Channel  │
│               │  │               │  │               │
│ ┌───────────┐ │  │ ┌───────────┐ │  │ ┌───────────┐ │
│ │ Message 1 │ │  │ │ Message 1 │ │  │ │ Message 1 │ │
│ ├───────────┤ │  │ ├───────────┤ │  │ ├───────────┤ │
│ │ Message 2 │ │  │ │ Message 2 │ │  │ │ Message 2 │ │
│ ├───────────┤ │  │ ├───────────┤ │  │ ├───────────┤ │
│ │ Message 3 │ │  │ │ Message 3 │ │  │ │ Message 3 │ │
│ └───────────┘ │  │ └───────────┘ │  │ └───────────┘ │
│               │  │               │  │               │
│ Capacity:     │  │ Capacity:     │  │ Capacity:     │
│ Unlimited*    │  │ Unlimited*    │  │ Unlimited*    │
└───────┬───────┘  └───────┬───────┘  └───────┬───────┘
        │                  │                  │
        │                  │                  │
        ▼                  ▼                  ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│ Writer Task   │  │ Writer Task   │  │ Writer Task   │
│ (Async)       │  │ (Async)       │  │ (Async)       │
│               │  │               │  │               │
│ loop {        │  │ loop {        │  │ loop {        │
│   msg = recv()│  │   msg = recv()│  │   msg = recv()│
│   serialize() │  │   serialize() │  │   serialize() │
│   ws.send()   │  │   ws.send()   │  │   ws.send()   │
│ }             │  │ }             │  │ }             │
└───────┬───────┘  └───────┬───────┘  └───────┬───────┘
        │                  │                  │
        │                  │                  │
        ▼                  ▼                  ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│ WebSocket     │  │ WebSocket     │  │ WebSocket     │
│ (TCP)         │  │ (TCP)         │  │ (TCP)         │
└───────┬───────┘  └───────┬───────┘  └───────┬───────┘
        │                  │                  │
        ▼                  ▼                  ▼
    Client 1           Client 2           Client 3


*PRODUCTION RECOMMENDATION: Use bounded channel
┌─────────────────────────────────────────────────────────┐
│ let (tx, rx) = mpsc::channel(1000);  // Bounded         │
│                                                          │
│ Benefits:                                                │
│ • Backpressure protection                               │
│ • Memory usage control                                  │
│ • Slow consumer detection                               │
│                                                          │
│ When full:                                               │
│ • Drop slow client OR                                   │
│ • Block sender (apply backpressure)                     │
└─────────────────────────────────────────────────────────┘
```

---

## 7. Multi-Instance Scaling with Redis

```
┌─────────────────────────────────────────────────────────────────────┐
│                  HORIZONTALLY SCALED ARCHITECTURE                    │
└─────────────────────────────────────────────────────────────────────┘

                          Internet
                             │
                             ▼
                    ┌────────────────┐
                    │ Load Balancer  │
                    │ (Sticky        │
                    │  Sessions)     │
                    └────────┬───────┘
                             │
          ┌──────────────────┼──────────────────┐
          │                  │                  │
          ▼                  ▼                  ▼
    ┌──────────┐       ┌──────────┐       ┌──────────┐
    │Instance 1│       │Instance 2│       │Instance 3│
    │          │       │          │       │          │
    │ 5K Conns │       │ 5K Conns │       │ 5K Conns │
    └─────┬────┘       └─────┬────┘       └─────┬────┘
          │                  │                  │
          │ Local            │ Local            │ Local
          │ Broadcast        │ Broadcast        │ Broadcast
          │                  │                  │
          ▼                  ▼                  ▼
    ┌──────────┐       ┌──────────┐       ┌──────────┐
    │ Event    │       │ Event    │       │ Event    │
    │Broadcast-│       │Broadcast-│       │Broadcast-│
    │ er       │       │ er       │       │ er       │
    └─────┬────┘       └─────┬────┘       └─────┬────┘
          │                  │                  │
          │ Publish          │ Publish          │ Publish
          │                  │                  │
          └──────────────────┼──────────────────┘
                             │
                             ▼
                    ┌────────────────┐
                    │  Redis Pub/Sub │
                    │                │
                    │  Channels:     │
                    │  • incidents   │
                    │  • alerts      │
                    │  • escalations │
                    │  • playbooks   │
                    └────────┬───────┘
                             │
          ┌──────────────────┼──────────────────┐
          │ Subscribe        │ Subscribe        │ Subscribe
          │                  │                  │
          ▼                  ▼                  ▼
    ┌──────────┐       ┌──────────┐       ┌──────────┐
    │ Redis    │       │ Redis    │       │ Redis    │
    │ Listener │       │ Listener │       │ Listener │
    └─────┬────┘       └─────┬────┘       └─────┬────┘
          │                  │                  │
          │ Forward          │ Forward          │ Forward
          │ to Local         │ to Local         │ to Local
          │                  │                  │
          ▼                  ▼                  ▼
    ┌──────────┐       ┌──────────┐       ┌──────────┐
    │ Local    │       │ Local    │       │ Local    │
    │ Broadcast│       │ Broadcast│       │ Broadcast│
    └─────┬────┘       └─────┬────┘       └─────┬────┘
          │                  │                  │
          │                  │                  │
          └──────────────────┼──────────────────┘
                             │
                             ▼
                    Connected Clients
                    (15,000 total)


EVENT FLOW:
──────────

1. Service publishes event
   └─► Any instance receives it

2. Instance publishes to Redis
   └─► All instances receive via Redis Pub/Sub

3. Each instance broadcasts locally
   └─► To connected WebSocket clients

4. Clients receive events
   └─► Regardless of which instance they're connected to


BENEFITS:
─────────

• No sticky session required (but recommended for performance)
• Event delivery to all clients across all instances
• Horizontal scaling to 100K+ connections
• High availability (instance failure doesn't affect others)
• Geographic distribution possible
```

---

## 8. Monitoring Dashboard Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│  WebSocket Streaming - Grafana Dashboard                            │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│  OVERVIEW                                                            │
├──────────────────────────┬──────────────────────┬───────────────────┤
│ Active Connections       │ Active Subscriptions │ Events/Second     │
│                          │                      │                   │
│      ┌─────────┐         │      ┌─────────┐    │    ┌─────────┐    │
│      │ 8,523   │         │      │ 12,459  │    │    │  1,245  │    │
│      └─────────┘         │      └─────────┘    │    └─────────┘    │
└──────────────────────────┴──────────────────────┴───────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│  CONNECTION METRICS                                                  │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Active Connections Over Time                                       │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │ 10K ┤                                    ╭────────           │    │
│  │     │                        ╭──────────╯                   │    │
│  │  5K ┤            ╭──────────╯                               │    │
│  │     │   ╭───────╯                                           │    │
│  │  0K └───┴──────────────────────────────────────────────────│    │
│  │      0h    6h    12h    18h    24h                          │    │
│  └────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  Connection Rate (per minute)                                       │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │ 100 ┤     ╭╮                                                │    │
│  │     │    ╭╯╰╮    ╭╮                                         │    │
│  │  50 ┤   ╭╯  ╰╮  ╭╯╰╮                                        │    │
│  │     │  ╭╯    ╰──╯  ╰╮                                       │    │
│  │   0 └──┴───────────────────────────────────────────────────│    │
│  │      0h    6h    12h    18h    24h                          │    │
│  └────────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│  EVENT DELIVERY                                                      │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Events Broadcast vs Delivered                                      │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │ 2K  ┤     Broadcast ────────                                │    │
│  │     │     Delivered ┄┄┄┄┄┄┄┄                                │    │
│  │ 1K  ┤     ╭─────────────────                                │    │
│  │     │   ╭╯┊┊┊┊┊┊┊┊┊┊┊┊┊┊┊┊                                  │    │
│  │  0  └───┴──────────────────────────────────────────────────│    │
│  │      0h    6h    12h    18h    24h                          │    │
│  └────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  Delivery Success Rate                                              │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │ 100%┤ ─────────────────────────────────────────────────    │    │
│  │  99%┤                                                       │    │
│  │  98%┤                                                       │    │
│  │  97%└──────────────────────────────────────────────────────│    │
│  │      0h    6h    12h    18h    24h                          │    │
│  └────────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│  LATENCY & PERFORMANCE                                               │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Message Latency Heatmap                                            │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │  P99 ┤ ░░░░░░░▓▓▓▓▓▒▒▒▒░░░░░░░░                            │    │
│  │  P95 ┤ ░░░░▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒▒░░░░░░                          │    │
│  │  P50 ┤ ░▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒▒▒▒▒░░                          │    │
│  │      └──────────────────────────────────────────────────   │    │
│  │       0h    6h    12h    18h    24h                         │    │
│  │                                                             │    │
│  │  Legend:                                                    │    │
│  │  ░ < 5ms   ▒ 5-10ms   ▓ 10-20ms   █ > 20ms                │    │
│  └────────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│  ERRORS & HEALTH                                                     │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Error Rate                                                          │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │  10 ┤                                                       │    │
│  │   5 ┤     ╭╮                                                │    │
│  │   1 ┤    ╭╯╰╮  ╭╮                                           │    │
│  │   0 └────┴──╰──╯╰─────────────────────────────────────────│    │
│  │      0h    6h    12h    18h    24h                          │    │
│  └────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  Broadcast Channel Saturation                                       │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │ 100%┤                                                       │    │
│  │  75%┤                                                       │    │
│  │  50%┤ ─────────────────────────────────────────────────    │    │
│  │  25%┤                                                       │    │
│  │   0%└──────────────────────────────────────────────────────│    │
│  │      0h    6h    12h    18h    24h                          │    │
│  └────────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│  ALERTS                                                              │
├──────────────────────────────────────────────────────────────────────┤
│  🔴 CRITICAL: Broadcast channel > 90% full                          │
│  🟡 WARNING:  Connection errors > 5/min                             │
│  🟢 OK:       All systems operational                               │
└──────────────────────────────────────────────────────────────────────┘
```

---

## Summary

This document provides visual representations of the WebSocket streaming architecture for the LLM Incident Manager. All diagrams use ASCII art for universal compatibility and can be viewed in any text editor or terminal.

**Key Visualizations:**
1. System Component Architecture - Overall system structure
2. Connection Lifecycle - Complete connection flow
3. Event Broadcasting - Event distribution mechanism
4. Event Filtering - Decision tree for event delivery
5. Session State Machine - Session lifecycle management
6. Message Queue Architecture - Per-connection message handling
7. Multi-Instance Scaling - Horizontal scaling with Redis
8. Monitoring Dashboard - Grafana visualization layout

**Related Documents:**
- `WEBSOCKET_ARCHITECT_DELIVERABLES.md` - Complete architecture specification
- `WEBSOCKET_QUICK_REFERENCE.md` - Quick start and API reference

**Status**: ✅ Production-Ready Architecture
