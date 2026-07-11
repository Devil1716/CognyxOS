# ADR 0003: SQLite WAL as the embedded event store

Status: Accepted. Date: 2026-07-11.

## Context and comparison

CognyxOS is offline-first, runs initially on a single local machine, and needs atomic append, durable replay, simple backup, and Windows/Linux/macOS support.

| Store      | Performance                            | Reliability/recovery                 | Cross-platform and complexity           | Storage/backup                   |
| ---------- | -------------------------------------- | ------------------------------------ | --------------------------------------- | -------------------------------- |
| SQLite WAL | strong local throughput, indexed reads | transactional, mature recovery       | bundled ecosystem; low complexity       | compact files; online backup API |
| BadgerDB   | high write throughput                  | application-managed recovery         | Go-centric, additional service boundary | LSM compaction overhead          |
| RocksDB    | very high throughput                   | mature but operationally complex     | native dependency burden on Windows     | tuning/compaction required       |
| LMDB       | fast reads, simple engine              | robust but single-writer constraints | binding/size-management complexity      | file-copy discipline             |

## Decision

Use SQLite in WAL mode through a single event-store service adapter. The store uses an append-only `events` table, durable subscriber cursors, and snapshots. SQLite is the persistence engine, not a public API. All access passes through the event-store contract.

The implementation will use `synchronous=FULL`, foreign keys, an explicit transaction for append plus outbox/cursor updates, and bounded connection ownership. Event payloads are JSON bytes validated before append; sensitive content is encrypted at rest using a future secret/key-management adapter.

## Operations policy

Retention follows [the event contract](../contracts/events.md): operational 30 days, audit/security 365 days, with compaction only after a valid snapshot and never for audit records. Replay reads immutable events from a durable cursor. Snapshots are versioned state materializations, include the final event cursor and schema version, and are validated before use. Startup restores the newest valid snapshot then replays later events.

Integrity checks run at startup and on diagnostics request. Corruption triggers read-only safe mode, copies database/WAL/SHM files for recovery, emits a security/audit record, restores the newest verified backup, and replays only validated events. No automatic destructive repair is allowed. Migrations are transactional, numbered, backup-gated, forward-only for production, and tested against representative historical databases. Backups use SQLite online backup, are encrypted, verified by restore test, and retained per local policy.
