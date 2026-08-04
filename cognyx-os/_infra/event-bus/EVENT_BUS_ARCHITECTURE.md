# CognyxOS Event Bus Architecture

## Overview
The Event Bus is the central nervous system of CognyxOS, enabling asynchronous communication between all layers and services. Built on NATS JetStream, it provides reliable, scalable messaging with exactly-once delivery semantics.

## Architecture

### Core Components

#### 1. NATS JetStream Cluster
- **Deployment**: 3-node cluster for high availability
- **Storage**: File-based persistent storage with replication
- **Location**: Layer 3 (Agent Kernel) with access from all layers

#### 2. Stream Types

**Command Stream** (`cognyx.commands`)
- Purpose: Request-response patterns for capability execution
- Retention: 24 hours
- Max message size: 10MB
- Subjects:
  - `cognyx.commands.capability.*`
  - `cognyx.commands.system.*`
  - `cognyx.commands.intent.*`

**Event Stream** (`cognyx.events`)
- Purpose: System-wide event broadcasting
- Retention: 7 days
- Max message size: 1MB
- Subjects:
  - `cognyx.events.lifecycle.*`
  - `cognyx.events.security.*`
  - `cognyx.events.resource.*`
  - `cognyx.events.notification.*`

**State Stream** (`cognyx.state`)
- Purpose: State synchronization across services
- Retention: Until snapshot
- Max message size: 5MB
- Subjects:
  - `cognyx.state.session.*`
  - `cognyx.state.global.*`
  - `cognyx.state.checkpoint.*`

**Audit Stream** (`cognyx.audit`)
- Purpose: Security audit logging
- Retention: 90 days (compliance)
- Max message size: 10KB
- Subjects:
  - `cognyx.audit.auth.*`
  - `cognyx.audit.permission.*`
  - `cognyx.audit.capability.*`

**Metrics Stream** (`cognyx.metrics`)
- Purpose: Performance and health metrics
- Retention: 30 days
- Max message size: 5KB
- Subjects:
  - `cognyx.metrics.performance.*`
  - `cognyx.metrics.health.*`
  - `cognyx.metrics.resource.*`

### Message Envelope

All messages follow a standard envelope format:

```json
{
  "id": "uuid-v4",
  "timestamp": "2024-01-15T10:30:00Z",
  "source": {
    "service": "capability-runtime",
    "instance": "cr-linux-01",
    "layer": 4
  },
  "destination": {
    "service": "agent-kernel",
    "instance": "*",
    "layer": 3
  },
  "subject": "cognyx.commands.capability.input.click",
  "correlation_id": "corr-uuid-v4",
  "causation_id": "cause-uuid-v4",
  "session_id": "session-uuid-v4",
  "trace_id": "otel-trace-id",
  "span_id": "otel-span-id",
  "priority": "normal",
  "ttl_seconds": 30,
  "payload": {}
}
```

### Priority Levels

| Level | Value | Use Case |
|-------|-------|----------|
| Critical | 0 | Security alerts, system failures |
| High | 1 | Real-time input processing |
| Normal | 2 | Standard capability execution |
| Low | 3 | Background tasks, logging |
| Deferred | 4 | Batch operations, sync |

## Communication Patterns

### Request-Reply Pattern

Used for synchronous capability execution:

```
Agent Kernel → [capability.request] → Capability Runtime
Agent Kernel ← [capability.response] ← Capability Runtime
```

**Timeouts**:
- Default: 30 seconds
- Real-time operations: 5 seconds
- Long-running operations: 300 seconds (with progress updates)

### Publish-Subscribe Pattern

Used for event broadcasting:

```
Service A → [event] → Event Bus → [event] → All Subscribers
```

### Stream Processing Pattern

Used for state synchronization:

```
Service → [state change] → Stream → Consumers (ordered processing)
```

## Service Integration

### Layer 3 (Agent Kernel)

**Publishes**:
- Intent parsing events
- Scheduling decisions
- State checkpoints
- Resource allocation commands

**Subscribes**:
- Capability execution results
- System health events
- User input events
- Security alerts

### Layer 4 (Capability Runtime)

**Publishes**:
- Capability execution results
- Adapter health status
- Resource utilization metrics
- Error conditions

**Subscribes**:
- Capability execution requests
- Configuration updates
- Health check commands
- Shutdown signals

### Layer 5 (Execution Runtimes)

**Publishes**:
- VM lifecycle events
- OS-specific capability results
- Resource availability updates
- Performance metrics

**Subscribes**:
- VM management commands
- Capability adapter instructions
- Configuration changes
- Cleanup commands

### Infrastructure Services

**Observability**:
- Subscribes to all metrics streams
- Publishes alert notifications
- Aggregates trace data

**Security**:
- Subscribes to audit streams
- Publishes authentication events
- Broadcasts security policy updates

**Configuration**:
- Subscribes to configuration change events
- Publishes validation results
- Broadcasts active configuration snapshots

## Quality of Service

### Delivery Guarantees

| Stream Type | Guarantee | Acknowledgment |
|-------------|-----------|----------------|
| Commands | At-least-once | Explicit ACK |
| Events | At-most-once | No ACK |
| State | Exactly-once | Idempotent processing |
| Audit | At-least-once | Persistent ACK |
| Metrics | At-most-once | No ACK |

### Ordering Guarantees

- **Per-subject ordering**: Messages with same subject maintain order
- **Per-session ordering**: Messages with same session_id maintain order
- **No global ordering**: Different subjects may arrive out of order

### Flow Control

- **Backpressure**: Consumer-controlled via acknowledgment rate
- **Rate limiting**: Configurable per-service quotas
- **Circuit breakers**: Automatic on repeated failures

## Security

### Authentication

- All connections require mTLS certificates
- Service identities validated against CA
- Certificate rotation every 24 hours

### Authorization

- Subject-based access control
- Per-service publish/subscribe permissions
- Dynamic permission updates via policy engine

### Encryption

- TLS 1.3 for all network traffic
- Payload encryption for sensitive data
- Key rotation every 7 days

## Scalability

### Horizontal Scaling

- Multiple NATS cluster nodes
- Shard streams by session_id for parallelism
- Consumer groups for load distribution

### Partitioning Strategy

```
Subject pattern: cognyx.{type}.{category}.{resource}.{session}
Example: cognyx.commands.capability.input.click.session-123
```

### Performance Targets

| Metric | Target |
|--------|--------|
| Latency (p50) | < 1ms |
| Latency (p99) | < 10ms |
| Throughput | 1M msg/sec |
| Message size | Up to 10MB |
| Connections | 10K concurrent |

## Monitoring

### Key Metrics

- **Message rates**: Published/consumed per subject
- **Latency**: End-to-end delivery time
- **Queue depth**: Pending messages per consumer
- **Error rates**: NACKs, timeouts, failures
- **Connection count**: Active clients

### Alerting Rules

- Message latency > 100ms (p99)
- Queue depth > 10K messages
- Error rate > 1%
- Connection drops > 10/min
- Disk usage > 80%

## Disaster Recovery

### Backup Strategy

- Continuous streaming to cold storage
- Daily snapshots of stream state
- Cross-region replication for critical streams

### Recovery Procedures

1. **Single node failure**: Automatic failover to remaining nodes
2. **Cluster failure**: Restore from latest snapshot
3. **Data corruption**: Rollback to last known good state
4. **Network partition**: Heal with conflict resolution

## Configuration Example

```yaml
nats:
  cluster:
    nodes:
      - nats-0.cognyx.internal:4222
      - nats-1.cognyx.internal:4222
      - nats-2.cognyx.internal:4222
  
  streams:
    commands:
      retention: 24h
      max_msg_size: 10485760
      replicas: 3
    events:
      retention: 168h
      max_msg_size: 1048576
      replicas: 3
    state:
      retention: until_snapshot
      max_msg_size: 5242880
      replicas: 3
    audit:
      retention: 2160h
      max_msg_size: 10240
      replicas: 3
    metrics:
      retention: 720h
      max_msg_size: 5120
      replicas: 2

  auth:
    tls:
      cert_file: /etc/cognyx/tls/service.crt
      key_file: /etc/cognyx/tls/service.key
      ca_file: /etc/cognyx/tls/ca.crt
    jwt:
      issuer: cognyx-event-bus
      audience: cognyx-services
```

## Best Practices

1. **Use correlation IDs**: Track request-response chains
2. **Set appropriate TTLs**: Prevent stale message processing
3. **Implement idempotency**: Handle duplicate deliveries
4. **Monitor queue depths**: Detect backpressure early
5. **Use structured payloads**: Enable schema validation
6. **Version your subjects**: Support backward compatibility
7. **Limit payload sizes**: Use references for large data
8. **Implement circuit breakers**: Prevent cascade failures
