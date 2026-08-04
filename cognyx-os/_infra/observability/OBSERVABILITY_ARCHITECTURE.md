# CognyxOS Observability Architecture

## Executive Summary

CognyxOS implements comprehensive observability using the OpenTelemetry framework, providing unified tracing, metrics, and logging across all six layers. Every operation is observable, traceable, and auditable.

## Core Principles

1. **Unified Telemetry**: Single framework for traces, metrics, and logs
2. **Zero Overhead**: Sampling strategies minimize performance impact
3. **Context Propagation**: Trace context flows through all layers
4. **Cardinality Control**: Managed metric dimensions prevent explosion
5. **Privacy by Design**: Sensitive data automatically redacted

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Observability Stack                       │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Tracing   │  │   Metrics   │  │       Logging       │  │
│  │  (Tempo)    │  │ (Prometheus)│  │       (Loki)        │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
│         │                │                     │             │
│         └────────────────┼─────────────────────┘             │
│                          │                                   │
│                 ┌────────▼────────┐                          │
│                 │  OpenTelemetry  │                          │
│                 │     Collector   │                          │
│                 └────────┬────────┘                          │
└──────────────────────────┼──────────────────────────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼────┐      ┌─────▼─────┐     ┌──────▼──────┐
   │ Layer 3 │      │  Layer 4  │     │   Layer 5   │
   │ Kernel  │      │ Capability│     │   Runtimes  │
   │ OTel SDK│      │  OTel SDK │     │   OTel SDK  │
   └─────────┘      └───────────┘     └─────────────┘
```

## Distributed Tracing

### Trace Context Propagation

Trace context propagates through all layers via W3C Trace Context:

```
Trace Context = {
  trace_id: "5bd66ef5095ebccd148dfa53",  // 128-bit, global trace
  span_id: "acff4103fa7b27d4",          // 64-bit, per operation
  trace_flags: "01",                     // Sampling decision
  trace_state: "cognyx=layer3;vendor=value"
}
```

### Span Structure

Each capability execution creates a span hierarchy:

```json
{
  "trace_id": "5bd66ef5095ebccd148dfa53",
  "span_id": "acff4103fa7b27d4",
  "parent_span_id": "836495038f2a4b1c",
  "name": "capability.input.click",
  "kind": "SPAN_KIND_INTERNAL",
  "start_time_unix_nano": 1704067200000000000,
  "end_time_unix_nano": 1704067200015000000,
  "attributes": {
    "cognyx.layer": "4",
    "cognyx.capability.category": "input",
    "cognyx.runtime.type": "linux",
    "cognyx.session.id": "session-uuid",
    "cognyx.intent.id": "intent-uuid",
    "input.x": 100,
    "input.y": 200,
    "input.button": "LEFT",
    "execution.status": "SUCCESS",
    "execution.duration_ms": 15
  },
  "events": [
    {
      "time_unix_nano": 1704067200005000000,
      "name": "permission_check",
      "attributes": {"result": "ALLOWED"}
    },
    {
      "time_unix_nano": 1704067200010000000,
      "name": "runtime_dispatch",
      "attributes": {"runtime": "linux-x11"}
    }
  ],
  "status": {"code": "STATUS_CODE_OK"}
}
```

### Instrumentation Points

**Layer 3 (Agent Kernel)**:
- Intent parsing spans
- Scheduling decision spans
- State management spans
- Resource allocation spans

**Layer 4 (Capability Runtime)**:
- Capability execution spans
- Adapter dispatch spans
- Permission check spans
- Result aggregation spans

**Layer 5 (Execution Runtimes)**:
- OS-specific operation spans
- VM communication spans
- Native API call spans
- Resource cleanup spans

### Sampling Strategies

```yaml
sampling:
  # Default: Sample 1% of traces
  default_ratio: 0.01
  
  # Always sample errors
  error_sampling:
    enabled: true
    ratio: 1.0
    
  # Always sample slow operations (>1s)
  rate_limiting:
    enabled: true
    threshold_ms: 1000
    ratio: 1.0
    
  # Sample specific capabilities at higher rates
  capability_based:
    input.click: 0.1      # 10%
    filesystem.write: 0.5 # 50% (security critical)
    system.*: 0.2         # 20%
    
  # Head-based sampling at source
  head_sampling:
    enabled: true
    sampler: "parent_based"
    
  # Tail-based sampling for complex decisions
  tail_sampling:
    enabled: true
    policies:
      - name: "error-policy"
        type: "status_code"
        status_codes: ["ERROR"]
      - name: "slow-policy"
        type: "latency"
        threshold_ms: 500
      - name: "probabilistic-policy"
        type: "probabilistic"
        sampling_percentage: 1
```

## Metrics Architecture

### Metric Types

**Counters** (monotonically increasing):
- `cognyx_capability_executions_total`
- `cognyx_errors_total`
- `cognyx_requests_total`

**Gauges** (current values):
- `cognyx_active_sessions`
- `cognyx_memory_usage_bytes`
- `cognyx_queue_depth`

**Histograms** (distribution):
- `cognyx_capability_duration_seconds`
- `cognyx_request_size_bytes`
- `cognyx_response_size_bytes`

**Summaries** (quantiles):
- `cognyx_latency_summary`

### Standard Labels

All metrics include standard labels:

```yaml
standard_labels:
  - service        # Service name (e.g., capability-runtime)
  - instance       # Instance identifier
  - layer          # Architecture layer (3, 4, 5, 6)
  - runtime_type   # Linux, Windows, macOS, Android, Cloud
  - version        # Service version
  - environment    # Production, staging, development
```

### Key Metrics

#### Capability Execution Metrics

```promql
# Total capability executions by type and result
cognyx_capability_executions_total{capability, runtime, result}

# Capability execution duration histogram
cognyx_capability_duration_seconds_bucket{capability, runtime, le}

# Active capability executions
cognyx_capability_executions_active{capability, runtime}

# Capability error rate
cognyx_capability_errors_total{capability, runtime, error_type}
```

#### Resource Metrics

```promql
# CPU usage by runtime
cognyx_runtime_cpu_usage_percent{runtime, instance}

# Memory usage by component
cognyx_memory_usage_bytes{component, layer}

# Disk I/O by runtime
cognyx_disk_io_bytes_total{runtime, operation}

# Network throughput
cognyx_network_bytes_total{direction, peer}
```

#### Session Metrics

```promql
# Active sessions
cognyx_active_sessions{user_type, session_state}

# Session duration
cognyx_session_duration_seconds{session_id}

# Intents per session
cognyx_intents_per_session{session_id}
```

### Recording Rules

Pre-computed aggregations for performance:

```yaml
groups:
  - name: cognyx_aggregations
    interval: 30s
    rules:
      - record: cognyx:capability_execution:rate5m
        expr: rate(cognyx_capability_executions_total[5m])
        
      - record: cognyx:capability_error:ratio5m
        expr: |
          sum(rate(cognyx_capability_errors_total[5m])) 
          / sum(rate(cognyx_capability_executions_total[5m]))
          
      - record: cognyx:capability_latency:p99_5m
        expr: |
          histogram_quantile(0.99, 
            rate(cognyx_capability_duration_seconds_bucket[5m]))
```

## Logging Architecture

### Log Structure

Structured JSON logs with consistent schema:

```json
{
  "timestamp": "2024-01-15T10:30:00.123456Z",
  "level": "INFO",
  "service": "capability-runtime",
  "instance": "cr-linux-01",
  "layer": 4,
  "trace_id": "5bd66ef5095ebccd148dfa53",
  "span_id": "acff4103fa7b27d4",
  "session_id": "session-uuid",
  "correlation_id": "corr-uuid",
  "message": "Capability executed successfully",
  "capability": "input.click",
  "runtime": "linux",
  "duration_ms": 15,
  "result": "SUCCESS",
  "attributes": {
    "x": 100,
    "y": 200,
    "button": "LEFT"
  },
  "caller": {
    "file": "input_adapter.go",
    "line": 142,
    "function": "Click"
  }
}
```

### Log Levels

| Level | Usage | Retention |
|-------|-------|-----------|
| TRACE | Detailed debugging | 1 hour |
| DEBUG | Development debugging | 6 hours |
| INFO | Normal operations | 30 days |
| WARN | Potential issues | 90 days |
| ERROR | Errors requiring attention | 1 year |
| FATAL | Critical failures | 7 years |

### Log Collection

```yaml
fluentbit_config:
  inputs:
    - name: tail
      path: /var/log/cognyx/*.log
      parser: json
      
  filters:
    - name: modify
      add_labels:
        environment: production
        cluster: cognyx-prod
        
    - name: grep
      regex: level (INFO|WARN|ERROR|FATAL)
      
    - name: lua
      script: redact_sensitive_data.lua
      
  outputs:
    - name: loki
      host: loki.cognyx.internal
      port: 3100
      labels:
        - service
        - layer
        - level
      line_format: json
```

### Sensitive Data Redaction

Automatic redaction patterns:

```lua
-- redact_sensitive_data.lua
function redact(record)
  -- Redact PII patterns
  record.message = record.message:gsub(
    "\\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Z|a-z]{2,}\\b", 
    "[EMAIL_REDACTED]"
  )
  
  -- Redact file paths containing sensitive directories
  record.message = record.message:gsub(
    "/home/[^/]+/", 
    "/home/[USER]/"
  )
  
  -- Redact credentials
  record.attributes.password = "[REDACTED]"
  record.attributes.token = "[REDACTED]"
  record.attributes.api_key = "[REDACTED]"
  
  return true, record
end
```

## Session Recording

### Recording Architecture

Full session capture for debugging and audit:

```
Session Recorder → {
  Screen captures (configurable FPS)
  Input events (clicks, keystrokes)
  Capability calls (with parameters)
  System state snapshots
  Audio recording (optional)
}
```

### Storage Format

```yaml
session_recording:
  format: "webm"  # Video
  metadata_format: "protobuf"
  
  video:
    codec: "vp9"
    fps: 5  # Configurable
    quality: "medium"
    resolution: "native"
    
  metadata:
    include_input_events: true
    include_capability_calls: true
    include_system_state: true
    include_audio: false  # Privacy default
    
  storage:
    location: "encrypted-object-storage"
    retention_days: 30
    encryption: "AES-256-GCM"
    access_control: "role-based"
```

### Privacy Controls

```yaml
privacy:
  # Automatic blurring of sensitive regions
  blur_regions:
    - password_fields: true
    - credit_card_inputs: true
    - personal_documents: true
    
  # Redaction of sensitive capability parameters
  redact_parameters:
    - capability: "input.type"
      fields: ["text"]
      condition: "contains_credentials"
      
    - capability: "filesystem.read"
      fields: ["content"]
      condition: "path_matches_sensitive_patterns"
      
  # User consent requirements
  consent:
    required_for_recording: true
    granular_options:
      - screen_capture
      - input_events
      - audio_recording
```

## Dashboards & Visualization

### Standard Dashboards

**System Health Dashboard**:
- Overall system status
- Error rates by layer
- Latency percentiles
- Resource utilization
- Active sessions

**Capability Performance Dashboard**:
- Execution counts by capability
- Duration distributions
- Error breakdown by type
- Runtime comparison
- Trend analysis

**Security Dashboard**:
- Authentication failures
- Authorization denials
- Policy violations
- Anomaly detections
- Audit log summary

**Resource Utilization Dashboard**:
- CPU/Memory by runtime
- Disk I/O patterns
- Network throughput
- VM resource allocation
- Capacity trends

### Alerting Rules

```yaml
alerting:
  groups:
    - name: cognyx_critical
      rules:
        - alert: HighCapabilityErrorRate
          expr: |
            cognyx:capability_error:ratio5m > 0.05
          for: 5m
          labels:
            severity: critical
          annotations:
            summary: "High capability error rate detected"
            description: "Error rate is {{ $value | humanizePercentage }}"
            
        - alert: HighLatency
          expr: |
            cognyx:capability_latency:p99_5m > 1.0
          for: 10m
          labels:
            severity: warning
          annotations:
            summary: "High capability latency"
            description: "P99 latency is {{ $value }}s"
            
        - alert: SessionBacklog
          expr: |
            cognyx_queue_depth > 10000
          for: 5m
          labels:
            severity: warning
          annotations:
            summary: "Session processing backlog"
            
        - alert: RuntimeUnavailable
          expr: |
            up{job="cognyx-runtimes"} == 0
          for: 1m
          labels:
            severity: critical
          annotations:
            summary: "Runtime instance unavailable"
```

## Performance Optimization

### Batching Strategies

```yaml
batching:
  traces:
    max_batch_size: 100
    max_delay_ms: 1000
    compression: "gzip"
    
  metrics:
    max_batch_size: 500
    max_delay_ms: 5000
    
  logs:
    max_batch_size: 200
    max_delay_ms: 2000
```

### Buffer Configuration

```yaml
buffers:
  memory:
    max_size_mb: 512
    flush_threshold_percent: 80
    
  disk:
    enabled: true
    max_size_gb: 10
    directory: /var/spool/otel-collector
```

### Sampling at Scale

```yaml
adaptive_sampling:
  enabled: true
  
  # Adjust sampling rate based on load
  load_based:
    target_throughput: 10000  # spans/sec
    adjustment_interval: 60s
    
  # Increase sampling during anomalies
  anomaly_based:
    increase_on_error: true
    increase_factor: 10
    duration_minutes: 15
```

## Integration Points

### External Systems

**Incident Management**:
- PagerDuty integration for critical alerts
- ServiceNow ticket creation
- Slack notifications for team channels

**Data Export**:
- S3/GCS for long-term storage
- Elasticsearch for advanced search
- BigQuery for analytics

**ML/AI Integration**:
- Anomaly detection models
- Predictive alerting
- Root cause analysis automation

### API Endpoints

```yaml
observability_apis:
  traces:
    query: "GET /api/v1/traces"
    search: "POST /api/v1/traces/search"
    
  metrics:
    query: "GET /api/v1/query"
    range: "GET /api/v1/query_range"
    labels: "GET /api/v1/labels"
    
  logs:
    query: "POST /loki/api/v1/query_range"
    labels: "GET /loki/api/v1/labels"
    
  sessions:
    list: "GET /api/v1/sessions"
    get: "GET /api/v1/sessions/{id}"
    recording: "GET /api/v1/sessions/{id}/recording"
```

## Compliance & Governance

### Data Retention

| Data Type | Production | Compliance | Archive |
|-----------|------------|------------|---------|
| Traces | 30 days | 90 days | 1 year |
| Metrics | 90 days | 1 year | 5 years |
| Logs | 30 days | 90 days | 7 years |
| Sessions | 7 days | 30 days | 1 year |
| Audit Logs | 90 days | 7 years | Permanent |

### Access Control

```yaml
rbac:
  roles:
    - name: observer
      permissions:
        - read:traces
        - read:metrics
        - read:logs
        
    - name: analyst
      permissions:
        - read:*
        - query:sessions
        - export:data
        
    - name: admin
      permissions:
        - "*:*"
        
  audit_access:
    enabled: true
    log_all_queries: true
```

### Cost Management

```yaml
cost_optimization:
  # Drop low-value telemetry
  drop_rules:
    - health_check_spans: true
    - debug_level_logs: true
    
  # Compress old data
  compression:
    after_days: 7
    algorithm: "zstd"
    
  # Downsample historical data
  downsampling:
    after_days: 30
    resolution: "5m"
    
  after_days: 90
    resolution: "1h"
```
