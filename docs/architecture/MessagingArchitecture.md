# CognyxOS Messaging & IPC Architecture

> **Document ID:** ARCH-006
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Core Platform Team

---

## Table of Contents

1. [Messaging Principles](#messaging-principles)
2. [Secure Message Bus](#secure-message-bus)
3. [Four Communication Patterns](#four-communication-patterns)
4. [Event Bus](#event-bus)
5. [Command Bus](#command-bus)
6. [Request/Response Pattern](#requestresponse-pattern)
7. [Streaming Messages](#streaming-messages)
8. [Cancellation Protocol](#cancellation-protocol)
9. [Timeout & Deadline Propagation](#timeout--deadline-propagation)
10. [Retry Policies](#retry-policies)
11. [IPC Framework Design](#ipc-framework-design)
12. [Message Bus Diagram](#message-bus-diagram)
13. [Wire Protocol Specification](#wire-protocol-specification)

---

## Messaging Principles

### 1. Mediated, Never Direct

No two modules communicate directly. Every byte crosses the Message Bus. This enables:
- Uniform capability checking
- Centralized audit
- Distributed tracing
- Back-pressure and flow control
- Message replay and debugging

### 2. Typed, Schema-Validated Messages

Every message payload has a `.proto` schema. Ad-hoc JSON is not accepted on the bus.
- Schema versioned via Protobuf compatibility rules
- Consumers can validate messages schematically before processing
- Bus validates payload against schema at ingress

### 3. Asynchronous by Default

All patterns are async under the hood. Sync wrappers are convenience APIs implemented in SDK.

### 4. Causality is First-Class

Every message carries:
- `correlation_id`: Identifies the user-facing action this message belongs to
- `causation_id`: Identifies the DIRECT PREVIOUS message that caused this one
- `trace_id`: W3C-compliant trace for distributed tracing

This enables **full reconstruction** of "why did X happen?" for any event in the system.

---

## Secure Message Bus

The Message Bus is the central nervous system of CognyxOS. It is implemented as a single privileged process (`cognyxos-bus`) supervised by PID 1.

### Bus Components

```
cognyxos-bus
├── Connection Router        Accepts UDS connections; peercred-based auth
├── Module Registry          Tracks identity → channel mappings, liveness pings
├── Capability Validator     Per-message signature + token check (inline)
├── Policy Evaluator         OPA Rego call (inline, fast path JIT cached)
├── Topic Router             pub/sub topic matching via Radix tree
├── Priority Queue           8 priority levels, per-sender fairness
├── Persistent WAL           Write-ahead log for exactly-once delivery
├── Stream Manager           Back-pressure, windowed acks for streams
├── Audit Appender           Every message → hash-chained audit entry
└── Dead-Letter Queue        Messages that can't be delivered after retries
```

### Connection Lifecycle

```
1. Module opens UDS connection to /run/cognyxos/bus.sock
2. Bus reads SO_PEERCRED → PID + UID + GID of connecting process
3. Bus queries Process Manager: "Does PID X belong to Identity Y?"
4. Bus issues 32-byte random nonce challenge
5. Module signs nonce with module Ed25519 identity key (held in-process)
6. Bus verifies signature against pre-registered public key (signed at install)
7. Bus assigns session_id, sends module capabilities available to this identity
8. Connection established. All further messages inherit this identity context.
9. Heartbeat PING/PONG every 1s. Miss 3 = session dead, resources reclaimed.
```

### Message Delivery Guarantees per Pattern

| Pattern | Delivery | Ordering | Duplicates? |
|---------|----------|----------|-------------|
| Command (QUEUED) | Exactly-Once | Per sender, per target strict | None |
| Event (PUBSUB) | At-Least-Once | Per-topic ordered | Possible on crash-recovery |
| Query (REQ/RESP) | At-Most-Once | N/A (request/response) | None (timeout fires) |
| Stream | Exactly-Once (byte-wise) | Strict per stream | None via seq+ack |

---

## Four Communication Patterns

```mermaid
graph LR
    subgraph Messaging Patterns
        CMD[Command Bus<br/>Exactly-once<br/>State-changing actions]
        Q[Request/Response<br/>At-most-once<br/>Read queries]
        EVT[Event Bus<br/>At-least-once<br/>State change announcements]
        STM[Streams<br/>Exactly-once (bytes)<br/>Bulk data transfer]
    end

    CMD -->|"Send(task)"| SVC[Target Service]
    SVC -->|"Ack(handle)"| CALLER1[Caller]

    Q -->|"Query(req)"| SVC2[Target Service]
    SVC2 -->|"Response(data)"| CALLER2[Caller]

    EVT -->|"Publish(topic, event)"| TOPIC[Topic Router]
    TOPIC -->|"Fan-out"| SUB1[Subscriber A]
    TOPIC -->|"Fan-out"| SUB2[Subscriber B]
    TOPIC -->|"Fan-out"| SUB3[Subscriber C]

    STM -->|"Open(target)"| HANDSHAKE[Stream Negotiation]
    HANDSHAKE -->|"Bidirectional data<br/>+ seq/ack + flow control"| ENDPOINTS[Both endpoints]
```

---

## Event Bus

### Purpose

**Publish/Subscribe pattern for announcing facts that have happened** (past tense events). Any module may publish; any module may subscribe.

### Event Naming Convention

Past tense, hierarchical dot notation:
```
{domain}.{aggregate}.{event_past_tense}
    ↓
filesystem.file.created
workspace.activated
ai.plan.step_completed
task.scheduler.deadline_missed
notification.user.dismissed
permission.capability.revoked
```

### Wildcard Subscriptions

```
filesystem.file.*             → all file events
filesystem.**.deleted         → all "deleted" events in filesystem tree
**                            → DANGEROUS (requires explicit cap: bus.subscribe_all)
```

### Event Schema (Envelope + Payload)

```protobuf
message EventEnvelope {
  common.Uuid event_id = 1;                    // UUID v7 (time-ordered)
  string topic = 2;                            // e.g. "fs.file.created"
  google.protobuf.Timestamp occurred_at = 3;   // When the fact happened
  common.IdentityId publisher = 4;             // Who published
  optional common.Uuid workspace_id = 5;       // Scope (if workspace-scoped)
  common.Uuid correlation_id = 6;              // User-action context
  common.Uuid causation_id = 7;                // Message causing this event
  google.protobuf.Any payload = 8;             // Typed event (protos under cognyx.events.*)
  map<string, string> indexing_tags = 9;       // Fast-path filtering tags
  bytes publisher_signature = 10;              // Ed25519 over fields 1-8

  // Persistence metadata (filled by bus)
  uint64 global_offset = 11;                   // WAL offset
  repeated uint64 consumer_offsets = 12;       // Per-subscriber acks
}
```

### Subscription Modes

| Mode | Interface | Semantics |
|------|-----------|-----------|
| **Ephemeral Push** | `subscribe(Filter) → Stream<Event>` | In-memory; subscriber disconnect → events dropped |
| **Durable Push** | `subscribe_durable(name, Filter)` | Persistent cursor; reconnect resumes |
| **Pull (batch)** | `read_events(cursor, limit)` | Batch pull for high-throughput consumers |

---

## Command Bus

### Purpose

**Request an action that changes state** (future imperative). Commands are delivered exactly once to a specific target module, processed in order per target.

### Command Naming Convention

Imperative verb, target module:
```
{module}.do_{action}
    ↓
filesystem.do_delete_file
workspace.do_activate
process.do_spawn
scheduler.do_submit_task
```

### Command Execution Model

```
Caller                  Bus                     Target
  │  SubmitCommand(cmd)   │                         │
  │──────────────────────►│                         │
  │                       │  Validate cap/policy     │
  │                       │  ──────────────────      │
  │                       │  Enqueue per-target Q   │
  │                       │                         │
  │                       │──── Deliver ───────────►│
  │                       │                         │ Execute
  │                       │                         │ ────────
  │                       │◄── Ack/receipt handle ──│ (async exec)
  │◄── CommandHandle ─────│                         │
  │                       │                         │
  │ [later]               │                         │
  │  GetStatus(handle)    │                         │
  │──────────────────────►│──── Query target ──────►│
  │◄── Status ────────────│◄── Result ──────────────│
  │                       │                         │
  │ OR                    │                         │
  │ Watch(handle)────────►│──── Subscribe to events►│
  │◄── stream of updates ─│◄── Progress events ─────│
```

### Command Retry & Idempotency

Every command carries `meta.request_id` (client-generated UUID).
- Bus uses request_id for deduplication: same target + same request_id within 24h = replay original response, no re-execution
- Retry policy on the COMMAND is set by the caller; bus executes it

```protobuf
message RetryPolicy {
  uint32 max_attempts = 1;           // 0 = no retry; default 3 for transient errors
  google.protobuf.Duration initial_backoff = 2;  // 100ms default
  double backoff_multiplier = 3;     // 2x default
  google.protobuf.Duration max_backoff = 4;      // 10s default
  repeated Error.ErrorCode retryable_errors = 5; // RETRYABLE set
}
```

---

## Request/Response Pattern

### Purpose

Read-only queries that **must not mutate state**. Synchronous convenience wrapper over async pattern.

**Key distinction from Commands:**
- Queries = `GetFoo(Request) → Foo` (idempotent, fast, no side effects)
- Commands = `DoFoo(Request) → Handle` (returns handle, async, side effects)

### Idempotency & Cache

Queries are safe to retry, safe to cache:
- Bus caches Query responses by (identity + query_sha256) for 5s by default
- Per-query TTL cache set via metadata

---

## Streaming Messages

### Purpose

Transfer bulk data between modules: file contents, GPU buffers, video frames, log streams, LLM token streams.

### Stream Lifecycle Protocol

```
Initiator                          Bus                           Responder
  │  OpenStream(target, spec)       │                              │
  │────────────────────────────────►│                              │
  │                                 │  Validate caps + flow ctrl    │
  │                                 │──────────────────────────    │
  │                                 │  Open stream_id              │
  │                                 │─────────────────────────────►│  StreamAccept
  │ StreamReady(stream_id, params)  │◄──── Stream params ──────────│
  │◄────────────────────────────────│                              │
  │                                 │                              │
  │  Data(stream_id, seq, bytes)    │                              │
  │────────────────────────────────►│  Flow control (credit based) │
  │                                 │  ────────  ───────────────── │
  │                                 │─────────────────────────────►│
  │                                 │◄── Ack(seq, credit) ─────────│
  │◄── Ack(seq, credit_balance) ────│                              │
  │                                 │                              │
  │  ... more data ...              │                              │
  │                                 │                              │
  │  Close(stream_id, reason, ok?)  │                              │
  │────────────────────────────────►│  Both close half, flush,     │
  │                                 │  deallocate credit           │
  │◄── CloseAck(final_seq) ─────────│◄── CloseAck ────────────────│
```

### Stream Flow Control: Credit-Based

Instead of TCP-like window:
- Responder grants `initial_credit = N` bytes at stream open
- Initiator sends ≤ credit bytes; decrements as sends
- Responder acks `ack_seq = X, additional_credit = Y`; initiator adds to balance
- If initiator runs out of credit: BLOCK until new Ack arrives with additional credit
- Prevents fast sender overwhelming slow receiver (e.g. UI shell reading 10GB file)

### Stream + Zero-Copy

For payloads >64KB, use **memfd + SCM_RIGHTS** (see IPC Framework):
- Stream metadata indicates "FD mode"
- Data path skips bus entirely; bus only mediates FD handoff via SCM_RIGHTS with capability token attached
- Bus never touches the bytes; just validates capability and passes the pre-sealed memfd

---

## Cancellation Protocol

Every long-lived operation (Commands, Queries, Streams, Plan execution) supports cooperative cancellation.

### Cancellation Reasons

```protobuf
enum CancelReason {
  USER_REQUESTED = 0;       // User clicked cancel
  TIMEOUT = 1;              // Deadline exceeded
  SUPERSEDED = 2;           // Newer operation replaces this one
  ERROR_UPSTREAM = 3;       // Dependency failed; no point continuing
  SHUTTING_DOWN = 4;        // Module or system shutting down
  RESOURCE_EVICTED = 5;     // Memory pressure; lowest-priority tasks cancelled
}
```

### Cancellation Flow

```
Caller                     Bus                      Target
  │ Cancel(handle, reason)  │                         │
  │────────────────────────►│                         │
  │                         │ Mark cancelling state   │
  │                         │────────────────────     │
  │                         │ CancelMsg delivered     │
  │                         │────────────────────────►│
  │                         │                         │ ├─ Stop producing new work
  │                         │                         │ ├─ Free resources
  │                         │                         │ └─ Rollback partially-done work (if transactional)
  │                         │◄── Cancelled(final_st) ─│
  │◄── CancelledAck ────────│                         │
```

**Important:** Cancellation is COOPERATIVE. Target must handle CancelMsg; bus will forcibly kill target's worker thread/process only after grace period.

---

## Timeout & Deadline Propagation

Deadlines are absolute timestamps, never "N ms from now."

Every message envelope carries `meta.deadline` (google.protobuf.Timestamp).

### Deadline Bubbling Rule

If operation A depends on operation B, B's deadline = min(A's deadline, B's own SLA max).

Example:
```
User request: "delete 1000 files", deadline = T + 30s
  →  Command: fs.delete_batch, deadline T + 29s (1s buffer subtracted)
      →  Per file: fs.delete, deadline = min( batch_deadline, now + 2s )
```

When a message arrives at a target past its deadline:
- Target immediately sends back `DEADLINE_EXCEEDED` error
- No work executed
- Upstream caller notified via correlation

---

## Retry Policies

### Built-in Retry Presets

```rust
// Transient failures: network blip, service restarting, lock contention
const RETRY_TRANSIENT: RetryPolicy = RetryPolicy {
  max_attempts: 3,
  initial_backoff: 100ms,
  backoff_multiplier: 2.0,
  max_backoff: 2s,
  retryable_errors: [ UNAVAILABLE, CONFLICT, INTERNAL_TRANSIENT ],
};

// Long-running async setup (e.g. VM boot, container pull)
const RETRY_ASYNC_SETUP: RetryPolicy = RetryPolicy {
  max_attempts: 5,
  initial_backoff: 500ms,
  backoff_multiplier: 2.5,
  max_backoff: 30s,
  retryable_errors: [ RESOURCE_EXHAUSTED, UNAVAILABLE ],
};

// No retry: user visible operations where double-exec unsafe
const RETRY_NEVER: RetryPolicy = RetryPolicy { max_attempts: 0 };
```

### Retry Jitter

All backoff values use exponential backoff with FULL jitter:
```
delay = min( max_backoff, initial_backoff * (multiplier^(attempt-1)) )
sleep = random_uniform(0, delay)
```
Avoids thundering herd on restart.

---

## IPC Framework Design

### Low-Level Transport Layers

```
┌─────────────────────────────────────────────────────────────┐
│  Application Layer (gRPC / Protocol Buffers)                │
├─────────────────────────────────────────────────────────────┤
│  Message Bus (pattern routing + cap validation)              │
├─────────────────────────────────────────────────────────────┤
│  IPC Broker (FD management + authentication)                 │
├──────────────────────────────┬──────────────────────────────┤
│  Small Messages  (<=64KB)    │  Large Messages   (>64KB)    │
│  Unix Domain Sockets         │  memfd_create + sealing      │
│  SOCK_SEQPACKET               │  SCM_RIGHTS FD passing      │
│  SO_PEERCRED auth             │  Zero-copy: mmap on recv    │
│  Kernel-level queuing         │  Sender caps in ancillary   │
├──────────────────────────────┴──────────────────────────────┤
│  Linux Kernel                                                │
└─────────────────────────────────────────────────────────────┘
```

### Unix Socket Authentication

Every message received:
1. Kernel populates `struct ucred` via SO_PEERCRED
2. Cross-referenced with Process Manager's ProcessRecord table
3. If PID doesn't match what Process Manager recorded → socket reset + audit event

### Secure Zero-Copy Path

```
SENDER:
  1. memfd_create("cognyx_ipc_payload", MFD_ALLOW_SEALING)
  2. ftruncate to payload_size
  3. mmap, write payload
  4. fcntl F_ADD_SEALS: SHRINK | WRITE | GROW | SEAL
  5. Send on UDS via sendmsg(): 1 header byte + SCM_RIGHTS with 1 FD
  6. munmap, close(fd)

BUS:
  1. recvmsg() → get header + FD
  2. verify seals: F_SEAL_SHRINK | F_SEAL_WRITE | F_SEAL_GROW
     (prevent sender from modifying after handoff)
  3. Validate capability token in header
  4. Sendmsg() forward FD to RECIPIENT (no mmap, no copy)

RECIPIENT:
  1. recvmsg() → header + FD
  2. fstat → size
  3. mmap(, PROT_READ, MAP_SHARED, fd, 0) → read data
  4. munmap, close(fd)

TOTAL COPIES: 1 (SENDER write only). Bus & Recipient zero copy via mmap.
```

---

## Message Bus Diagram

```mermaid
graph TB
    subgraph Senders
        S1[Module A<br/>identity:a]
        S2[Module B<br/>identity:b]
        S3[Agent C<br/>identity:c]
    end

    subgraph Message_Bus_Process ["cognyxos-bus (PID supervised by init)"]
        direction TB
        AUTH[1. Auth Layer<br/>peercred + Ed25519 signed nonce]
        ING[2. Ingress Schema Validation<br/>Proto parse + valid]
        CAP[3. Capability Token Verify<br/>sig + not revoked + not expired]
        POL[4. Policy Engine (OPA JIT)<br/>Rego evaluate → ALLOW/DENY/HITL_ESCALATE]
        QUE[5. Priority Queue<br/>8 levels, WFQ per source]
        ROUT[6. Target / Topic Router<br/>per-target queues<br/>per-topic fanout]
        AUD[7. Audit Appender<br/>hash-chained signed entry]
        OUT[8. Egress + Flow Ctrl<br/>Credit check + deadline]
        WAL[9. Persistent WAL<br/>Commands + Durable Events<br/>sqlite WAL]
        DLQ[10. Dead Letter Queue<br/>After retries exhausted]
    end

    subgraph Receivers
        R1[Module X]
        R2[Module Y]
        R3[Subscriber Z]
        R4[Audit Service]
    end

    S1 & S2 & S3 --> AUTH --> ING --> CAP --> POL
    POL -->|ALLOW| QUE --> ROUT
    POL -->|DENY| DROP[DROP + error response]
    POL -->|HITL| PAUSE[Pause flow<br/>escalate]
    ROUT -->|Cmd| R1 & R2
    ROUT -->|Event Fan-out| R3
    CAP & POL & ROUT --> AUD --> R4
    QUE -->|Durable| WAL
    ROUT -->|Exhausted retries| DLQ
```

---

## Wire Protocol Specification

### Binary Framing (UDS Transport, Small Messages)

All messages use length-prefixed frames. This is the on-the-wire format BEFORE Protobuf serialization of the `MessageEnvelope`:

```
┌──────────────────────────────────────────────────────────────────┐
│                     MESSAGE FRAME (little-endian)                 │
├────────┬──────────┬───────────┬───────────┬──────────────────────┤
│ MAGIC  │ FRAME    │ PAYLOAD   │ ANCILLARY │ PADDING to 8          │
│ 4B     │ TYPE     │ SIZE      │ DATA SIZE │ (multiples of 8B)    │
│0x53534743│ 2B     │ 4B        │ 4B        │ N bytes              │
│"CSGS"   │          │           │           │                      │
├────────┴──────────┴───────────┴───────────┼──────────────────────┤
│              PROTOBUF MESSAGE ENVELOPE     │ ANCILLARY (FD list)  │
│              PAYLOAD_SIZE bytes            │ ANCILLARY_DATA_SIZE  │
└────────────────────────────────────────────┴──────────────────────┘

Frame Types:
  0x0001 = MESSAGE
  0x0002 = PING
  0x0003 = PONG
  0x0004 = AUTH_CHALLENGE
  0x0005 = AUTH_RESPONSE
  0x0006 = FD_HANDOFF (preceding msg has FDs in ancillary)
  0x00FF = ERROR
```

### Protocol Versioning

Magic bytes encode major version in last byte of magic (currently 0x43 = v3 ASCII).
- Session negotiates version via AUTH handshake
- Backward compatibility: newer servers accept older clients with feature flags
