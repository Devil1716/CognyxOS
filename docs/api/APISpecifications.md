# CognyxOS API Specifications

> **Document ID:** API-001
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Platform API Team

---

## Table of Contents

1. [API Design Principles](#api-design-principles)
2. [Protocol Conventions](#protocol-conventions)
3. [Common Types & Error Codes](#common-types--error-codes)
4. [Workspace API](#workspace-api)
5. [Task API](#task-api)
6. [Agent API](#agent-api)
7. [Memory API](#memory-api)
8. [Filesystem API](#filesystem-api)
9. [Plugin API](#plugin-api)
10. [Notification API](#notification-api)
11. [Permission API](#permission-api)
12. [Context API](#context-api)
13. [Device API](#device-api)
14. [Window API](#window-api)
15. [Application API](#application-api)
16. [Container API](#container-api)
17. [VM API](#vm-api)
18. [Search API](#search-api)

---

## API Design Principles

1. **Capability First:** Every API method accepts a `capability_token` parameter (or carries it via gRPC metadata). There are NO ambient API calls.
2. **Three Protocol Equivalence:** Every API surface exists simultaneously as:
   - gRPC (canonical, high-performance, internal)
   - REST (HTTP/JSON, web clients)
   - GraphQL (flexible queries, UI development)
3. **Schema-Driven:** All APIs are generated from `.proto` files in `/proto/services/`. Hand-written wrappers are forbidden.
4. **Deterministic Errors:** All error responses are typed, with stable error codes. Errors include `suggested_user_action`, `retryable`, and `domain`.
5. **Idempotent:** Read operations are idempotent by definition. Write operations accept client-controlled `request_id` for exactly-once semantics.
6. **Pagination:** All list/query operations use cursor-based pagination with `page_size` + `page_token`, never offset/limit.

---

## Protocol Conventions

### gRPC Conventions
- Service name: `cognyx.{module}.v{major}.{ServiceName}`
- Streaming where appropriate: long-lived watch operations = server streaming
- Metadata: `x-cognyx-cap-token` (binary), `x-cognyx-correlation-id` (string), `x-cognyx-workspace-id` (string)
- Deadline: All calls require explicit client-side deadline.

### REST Conventions (generated via gRPC-Gateway)
- Base URL: `/api/v1/{module}/{resource}`
- Capability token in `Authorization: Bearer <cap-jwt>` header
- Correlation ID in `X-Correlation-ID` response header
- Problem+JSON (RFC 9457) for error bodies

### GraphQL Conventions (generated via GraphQL Gateway)
- Root Query: `query.workspace`, `query.task`, ...
- Root Mutation: `mutation.createWorkspace`, ...
- Subscriptions: `subscription.onWorkspaceEvent(workspace_id)`
- All resolvers validate capability tokens

---

## Common Types & Error Codes

### Standard Message Wrapper

```protobuf
syntax = "proto3";
package cognyx.common.v1;
import "google/protobuf/timestamp.proto";
import "google/protobuf/struct.proto";

message Uuid { string value = 1; }  // UUID v7 canonical string

message IdentityId { string value = 1; }

message CapabilityToken {
  bytes token_bytes = 1;
  Uuid token_id = 2;
}

message PaginationRequest {
  int32 page_size = 1;       // Default = 50; max = 1000
  string page_token = 2;     // Opaque cursor from previous response
}

message PaginationResponse {
  string next_page_token = 1;  // Empty = no more results
  int64 total_estimated = 2;   // Optional, approximate count
}

message RequestMetadata {
  Uuid request_id = 1;         // Client-generated, dedup key
  google.protobuf.Timestamp deadline = 2;
  IdentityId as_identity = 3;  // For sudo/impersonate (requires escalated cap)
  Uuid correlation_id = 4;
  Uuid causation_id = 5;
}

message Error {
  enum ErrorCode {
    UNKNOWN = 0;
    INVALID_ARGUMENT = 1;
    UNAUTHENTICATED = 2;
    PERMISSION_DENIED = 3;
    CAPABILITY_EXPIRED = 4;
    CAPABILITY_REVOKED = 5;
    NOT_FOUND = 6;
    ALREADY_EXISTS = 7;
    RESOURCE_EXHAUSTED = 8;
    RATE_LIMITED = 9;
    HITL_REQUIRED = 10;
    AUTH_ESCALATION_REQUIRED = 11;
    FAILED_PRECONDITION = 12;
    CONFLICT = 13;
    INTERNAL = 14;
    UNIMPLEMENTED = 15;
    SHUTTING_DOWN = 16;
    WORKSPACE_INACTIVE = 17;
  }
  ErrorCode code = 1;
  string message = 2;          // English, developer-readable
  string user_message_localized = 3;  // Optional, i18n user-facing
  string suggested_user_action = 4;   // Optional action suggestion
  bool retryable = 5;
  google.protobuf.Duration retry_after = 6;
  string domain = 7;          // e.g. "filesystem", "permissions"
  map<string, google.protobuf.Value> details = 8;
}
```

---

## 4. Workspace API

```protobuf
syntax = "proto3";
package cognyx.workspace.v1;
import "cognyx/common/v1/common.proto";

// ===== Resourc
message Workspace {
  common.Uuid id = 1;
  string name = 2;
  string description = 3;
  common.IdentityId owner = 4;
  repeated WorkspaceMember members = 5;
  ResourceQuota resource_quota = 6;
  google.protobuf.Timestamp created_at = 7;
  WorkspaceState state = 8;
  repeated string installed_capability_ids = 9;
  repeated string tags = 10;
  map<string, string> metadata = 11;
}

enum WorkspaceState {
  INACTIVE = 0;
  ACTIVATING = 1;
  ACTIVE = 2;
  HIBERNATING = 3;
  HIBERNATED = 4;
  ARCHIVING = 5;
  ARCHIVED = 6;
  DELETING = 7;
  ERROR = 8;
}

message WorkspaceMember {
  common.IdentityId identity = 1;
  WorkspaceRole role = 2;
  google.protobuf.Timestamp added_at = 3;
}

enum WorkspaceRole { OWNER = 0; ADMIN = 1; CONTRIBUTOR = 2; VIEWER = 3; }

message ResourceQuota {
  optional uint64 memory_limit_bytes = 1;
  optional uint32 cpu_cores_max = 2;
  optional uint64 disk_limit_bytes = 3;
  optional uint64 network_out_bps = 4;
  optional uint32 max_processes = 5;
}

// ===== RPCs =====
service WorkspaceService {
  // Create a new workspace (empty or from template)
  rpc CreateWorkspace(CreateWorkspaceRequest) returns (Workspace);

  // Activate (mount namespaces, start workspace services)
  rpc ActivateWorkspace(ActivateWorkspaceRequest) returns (ActivateWorkspaceResponse);

  // Hibernate (save state, unmount)
  rpc HibernateWorkspace(HibernateWorkspaceRequest) returns (HibernateWorkspaceResponse);

  // Delete workspace (and all its data)
  rpc DeleteWorkspace(DeleteWorkspaceRequest) returns (google.protobuf.Empty);

  // Clone an existing workspace into a new one
  rpc CloneWorkspace(CloneWorkspaceRequest) returns (Workspace);

  // Get a single workspace
  rpc GetWorkspace(GetWorkspaceRequest) returns (Workspace);

  // List workspaces visible to caller
  rpc ListWorkspaces(ListWorkspacesRequest) returns (ListWorkspacesResponse);

  // Update workspace metadata (name, description, tags, resource limits)
  rpc UpdateWorkspace(UpdateWorkspaceRequest) returns (Workspace);

  // Add or update workspace member role
  rpc SetMemberRole(SetMemberRoleRequest) returns (google.protobuf.Empty);

  // Remove member from workspace
  rpc RemoveMember(RemoveMemberRequest) returns (google.protobuf.Empty);

  // Watch workspace lifecycle events
  rpc WatchWorkspaceEvents(WatchWorkspaceEventsRequest) returns (stream WorkspaceEvent);

  // Export workspace as encrypted archive for backup/migration
  rpc ExportWorkspace(ExportWorkspaceRequest) returns (stream ExportChunk);

  // Import from archive
  rpc ImportWorkspace(stream ImportChunk) returns (ImportWorkspaceResponse);
}

// ===== Messages =====
message CreateWorkspaceRequest {
  common.RequestMetadata meta = 1;
  common.CapabilityToken cap = 2;
  string name = 3;
  string description = 4;
  optional ResourceQuota resource_quota = 5;
  optional common.Uuid template_workspace_id = 6;  // Clone template
  repeated string tags = 7;
  map<string, string> metadata = 8;
}

message ActivateWorkspaceRequest {
  common.RequestMetadata meta = 1;
  common.CapabilityToken cap = 2;
  common.Uuid workspace_id = 3;
  optional bool force = 4;  // Force if unclean shutdown
}

message ActivateWorkspaceResponse {
  Workspace workspace = 1;
  google.protobuf.Duration activation_duration = 2;
  repeated common.Error warnings = 3;
}

message GetWorkspaceRequest {
  common.RequestMetadata meta = 1;
  common.CapabilityToken cap = 2;
  common.Uuid workspace_id = 3;
}

message ListWorkspacesRequest {
  common.RequestMetadata meta = 1;
  common.CapabilityToken cap = 2;
  common.PaginationRequest pagination = 3;
  optional WorkspaceState filter_state = 4;
  optional string tag_filter = 5;
  optional string search_query = 6;
}

message ListWorkspacesResponse {
  repeated Workspace workspaces = 1;
  common.PaginationResponse pagination = 2;
}

message WorkspaceEvent {
  enum Type { CREATED = 0; ACTIVATED = 1; HIBERNATED = 2;
              UPDATED = 3; DELETED = 4; MEMBER_ADDED = 5; MEMBER_REMOVED = 6; }
  Type event_type = 1;
  common.Uuid workspace_id = 2;
  google.protobuf.Timestamp timestamp = 3;
  common.IdentityId actor = 4;
  optional Workspace snapshot = 5;
}
```

**REST Examples:**
```
GET    /api/v1/workspaces?state=ACTIVE&page_size=20
POST   /api/v1/workspaces
GET    /api/v1/workspaces/{id}
PATCH  /api/v1/workspaces/{id}
DELETE /api/v1/workspaces/{id}
POST   /api/v1/workspaces/{id}:activate
POST   /api/v1/workspaces/{id}:hibernate
POST   /api/v1/workspaces/{id}:clone
```

---

## 5. Task API

```protobuf
syntax = "proto3";
package cognyx.task.v1;

message ScheduledTask {
  common.Uuid task_id = 1;
  optional common.Uuid workspace_id = 2;
  common.IdentityId owner = 3;
  TaskType task_type = 4;
  uint32 priority = 5;  // 0 = highest
  optional google.protobuf.Timestamp deadline = 6;
  repeated common.Uuid depends_on = 7;
  TaskState state = 8;
  RetryPolicy retry_policy = 9;
  optional google.protobuf.Timestamp created_at = 10;
  optional google.protobuf.Timestamp started_at = 11;
  optional google.protobuf.Timestamp completed_at = 12;
  optional TaskResult result = 13;
  map<string, string> metadata = 14;
}

enum TaskType { AI_PLAN = 0; OS_INTERNAL = 1; USER_REQUEST = 2; PERIODIC = 3; }
enum TaskState { PENDING = 0; READY = 1; RUNNING = 2; SUSPENDED = 3;
                 COMPLETED = 4; FAILED = 5; CANCELLED = 6; DEADLINE_MISS = 7; }

message RetryPolicy {
  uint32 max_attempts = 1;
  google.protobuf.Duration initial_backoff = 2;
  double backoff_multiplier = 3;
  google.protobuf.Duration max_backoff = 4;
}

service TaskSchedulerService {
  rpc SubmitTask(SubmitTaskRequest) returns (ScheduledTask);
  rpc CancelTask(CancelTaskRequest) returns (google.protobuf.Empty);
  rpc SuspendTask(SuspendTaskRequest) returns (google.protobuf.Empty);
  rpc ResumeTask(ResumeTaskRequest) returns (google.protobuf.Empty);
  rpc GetTaskStatus(GetTaskStatusRequest) returns (ScheduledTask);
  rpc ListTasks(ListTasksRequest) returns (ListTasksResponse);
  rpc WatchTaskEvents(WatchTaskEventsRequest) returns (stream TaskEvent);
  rpc GetQueueStats(QueueStatsRequest) returns (QueueStats);
}
```

---

## 6. Agent API

```protobuf
syntax = "proto3";
package cognyx.agent.v1;

message Agent {
  common.Uuid agent_id = 1;
  string display_name = 2;
  AgentType type = 3;
  AgentExecutionModel execution_model = 4;
  repeated string declared_capability_ids = 5;
  optional common.Uuid workspace_id = 6;
  AgentState state = 7;
  map<string, string> metadata = 8;
}

enum AgentType { SYSTEM_BUILTIN = 0; USER_INSTALLED = 1;
                WORKSPACE_LOCAL = 2; REMOTE_FEDERATED = 3; }
enum AgentExecutionModel { PROCESS = 0; WASM = 1; REMOTE = 2; }
enum AgentState { STOPPED = 0; STARTING = 1; RUNNING = 2; PAUSED = 3; ERROR = 4; }

message AgentMessage {
  common.Uuid message_id = 1;
  oneof from {
    common.IdentityId user = 2;
    common.Uuid agent_id = 3;
  }
  oneof to {
    common.Uuid target_agent_id = 4;
    bool broadcast_all = 5;
    bool workspace_all = 6;
  }
  AgentMessageType message_type = 7;
  common.Uuid correlation_id = 8;
  optional common.CapabilityToken delegation_cap = 9;
  google.protobuf.Struct payload = 10;
  google.protobuf.Timestamp deadline = 11;
}

enum AgentMessageType {
  TASK_DELEGATION = 0; QUERY = 1; RESPONSE = 2;
  NOTIFICATION = 3; STATUS_UPDATE = 4; ERROR = 5;
}

service AgentOrchestratorService {
  rpc SpawnAgent(SpawnAgentRequest) returns (Agent);
  rpc TerminateAgent(TerminateAgentRequest) returns (google.protobuf.Empty);
  rpc SendMessage(SendMessageRequest) returns (SendReceipt);
  rpc QueryAgent(QueryAgentRequest) returns (AgentResponse);
  rpc ListAgents(ListAgentsRequest) returns (ListAgentsResponse);
  rpc GetAgentInfo(GetAgentInfoRequest) returns (Agent);
  rpc WatchAgentEvents(WatchAgentEventsRequest) returns (stream AgentEvent);
  rpc RegisterAgentManifest(RegisterManifestRequest) returns (ManifestRegistration);
}
```

---

## 7. Memory API

```protobuf
syntax = "proto3";
package cognyx.memory.v1;

message EpisodicRecord {
  common.Uuid id = 1;
  google.protobuf.Timestamp timestamp = 2;
  optional common.Uuid workspace_id = 3;
  EpisodicEventType event_type = 4;
  Actor actor = 5;
  string content_summary = 6;
  float importance_score = 7;
  repeated common.Uuid related_ids = 8;
}

message SemanticTriple {
  common.Uuid id = 1;
  string subject = 2;
  string predicate = 3;
  string object = 4;
  float confidence = 5;
  optional common.Uuid source_episodic_id = 6;
}

message Procedure {
  common.Uuid id = 1;
  string name = 2;
  string description = 3;
  uint32 usage_count = 4;
  float success_rate = 5;
  repeated string tags = 6;
}

service MemoryService {
  rpc StoreEpisodic(StoreEpisodicRequest) returns (common.Uuid);
  rpc StoreSemantic(StoreSemanticRequest) returns (common.Uuid);
  rpc StoreProcedure(StoreProcedureRequest) returns (common.Uuid);

  rpc RetrieveEpisodic(RetrieveRequest) returns (RetrieveEpisodicResponse);
  rpc RetrieveSemantic(SemanticQuery) returns (RetrieveSemanticResponse);
  rpc RetrieveProcedure(ProcedureQuery) returns (RetrieveProcedureResponse);
  rpc HybridRetrieve(HybridQuery) returns (HybridRetrieveResponse);

  rpc Forget(ForgetRequest) returns (google.protobuf.Empty);
  rpc ConsolidateNightly(ConsolidateRequest) returns (ConsolidationReport);
  rpc GetMemoryStats(MemoryStatsRequest) returns (MemoryStats);
}
```

---

## 8. Filesystem API

```protobuf
syntax = "proto3";
package cognyx.fs.v1;

message FileHandle { common.Uuid handle_id = 1; }

message FileMetadata {
  string path = 1;
  uint64 size_bytes = 2;
  FileKind kind = 3;
  uint32 permissions_unix = 4;
  common.IdentityId owner = 5;
  google.protobuf.Timestamp modified_at = 6;
  google.protobuf.Timestamp accessed_at = 7;
  google.protobuf.Timestamp created_at = 8;
  optional string mime_type = 9;
  map<string, string> extended_attributes = 10;
  map<string, string> user_tags = 11;
}

enum FileKind { REGULAR = 0; DIRECTORY = 1; SYMLINK = 2;
                CHAR_DEV = 3; BLOCK_DEV = 4; FIFO = 5; SOCKET = 6; }

message SnapshotInfo {
  common.Uuid snapshot_id = 1;
  string base_path = 2;
  uint64 size_bytes = 3;
  google.protobuf.Timestamp created_at = 4;
  bool read_only = 5;
}

service FilesystemService {
  rpc OpenFile(OpenFileRequest) returns (FileHandle);
  rpc ReadFile(ReadFileRequest) returns (stream FileChunk);
  rpc WriteFile(stream WriteFileRequest) returns (WriteResult);
  rpc StatFile(StatFileRequest) returns (FileMetadata);
  rpc ListDirectory(ListDirectoryRequest) returns (stream DirectoryEntry);
  rpc CreateDirectory(MkdirRequest) returns (google.protobuf.Empty);
  rpc DeleteFile(DeleteFileRequest) returns (google.protobuf.Empty);
  rpc MoveFile(MoveFileRequest) returns (google.protobuf.Empty);
  rpc CopyFile(CopyFileRequest) returns (CopyResult);
  rpc CreateSnapshot(CreateSnapshotRequest) returns (SnapshotInfo);
  rpc RestoreSnapshot(RestoreSnapshotRequest) returns (google.protobuf.Empty);
  rpc ListSnapshots(ListSnapshotsRequest) returns (ListSnapshotsResponse);
  rpc WatchPath(WatchPathRequest) returns (stream FilesystemEvent);
  rpc GetExtendedAttributes(XattrRequest) returns (XattrResponse);
  rpc SetExtendedAttributes(SetXattrRequest) returns (google.protobuf.Empty);
  rpc SearchByMetadata(SearchMetadataRequest) returns (stream FileMetadata);
}

message FilesystemEvent {
  enum Op { CREATED = 0; MODIFIED = 1; DELETED = 2;
            MOVED_FROM = 3; MOVED_TO = 4; ATTR_CHANGED = 5; }
  Op op = 1;
  string path = 2;
  optional string new_path = 3;
  common.IdentityId actor = 4;
  google.protobuf.Timestamp timestamp = 5;
}
```

---

## 9. Plugin API

```protobuf
syntax = "proto3";
package cognyx.plugin.v1;

message PluginManifest {
  string plugin_id = 1;            // Reverse-DNS: "com.example.foo"
  string display_name = 2;
  string version = 3;              // SemVer
  string min_os_version = 4;
  repeated PluginCapabilityDeclaration required_capabilities = 5;
  repeated PluginCapabilityDeclaration optional_capabilities = 6;
  repeated PluginEntryPoint entry_points = 7;
  repeated string tags = 8;
  string description = 9;
}

message PluginCapabilityDeclaration {
  string capability_namespace = 1;  // e.g. "filesystem.read"
  string resource_pattern = 2;
  string justification = 3;         // Shown to user at install
}

message PluginEntryPoint {
  string name = 1;
  PluginKind kind = 2;
  string wasm_module_sha256 = 3;
  repeated string arguments = 4;
}
enum PluginKind { UI_EXTENSION = 0; TOOL = 1; SEARCH_PROVIDER = 2;
                  EVENT_LISTENER = 3; COMMAND_HANDLER = 4; }

message PluginInstance {
  string plugin_id = 1;
  common.Uuid instance_id = 2;
  optional common.Uuid workspace_id = 3;
  PluginInstanceState state = 4;
  repeated string active_capability_ids = 5;
}
enum PluginInstanceState { STOPPED = 0; RUNNING = 1; PAUSED = 2; ERROR = 3; }

service PluginHostService {
  rpc InstallPlugin(InstallPluginRequest) returns (PluginManifest);
  rpc UninstallPlugin(UninstallPluginRequest) returns (google.protobuf.Empty);
  rpc EnablePlugin(EnablePluginRequest) returns (PluginInstance);
  rpc DisablePlugin(DisablePluginRequest) returns (google.protobuf.Empty);
  rpc ListInstalled(ListInstalledRequest) returns (ListInstalledResponse);
  rpc InvokePluginTool(InvokeToolRequest) returns (InvokeToolResponse);
  rpc SubscribePluginEvents(SubscribeEventsRequest) returns (stream PluginEvent);
  rpc GetPluginManifest(GetManifestRequest) returns (PluginManifest);
}
```

---

## 10. Notification API

```protobuf
syntax = "proto3";
package cognyx.notification.v1;

message Notification {
  common.Uuid id = 1;
  string title = 2;
  string body = 3;
  NotificationPriority priority = 4;
  optional common.Uuid workspace_id = 5;
  common.IdentityId recipient = 6;
  repeated NotificationAction actions = 7;
  optional string deep_link = 8;
  google.protobuf.Timestamp created_at = 9;
  optional google.protobuf.Timestamp read_at = 10;
  optional google.protobuf.Timestamp expires_at = 11;
  map<string, string> metadata = 12;
}

enum NotificationPriority {
  BACKGROUND = 0;   // Silent, no popup
  LOW = 1;          // Popup, no sound
  NORMAL = 2;       // Popup + sound (default)
  HIGH = 3;         // Popup + loud sound, top of list
  CRITICAL = 4;     // Fullscreen modal, cannot dismiss silently
}

message NotificationAction {
  string action_id = 1;
  string label = 2;
  NotificationActionStyle style = 3;
  optional google.protobuf.Struct payload = 4;
}
enum NotificationActionStyle { DEFAULT = 0; SUGGESTED = 1; DESTRUCTIVE = 2; }

service NotificationService {
  rpc SendNotification(SendNotificationRequest) returns (Notification);
  rpc DismissNotification(DismissRequest) returns (google.protobuf.Empty);
  rpc UpdateNotification(UpdateRequest) returns (Notification);
  rpc ListNotifications(ListRequest) returns (ListResponse);
  rpc MarkAsRead(MarkRequest) returns (google.protobuf.Empty);
  rpc MarkAllAsRead(MarkAllRequest) returns (google.protobuf.Empty);
  rpc RegisterDeliveryChannel(ChannelRequest) returns (ChannelId);
  rpc WatchNotifications(WatchRequest) returns (stream NotificationEvent);
}
```

---

## 11. Permission API

```protobuf
syntax = "proto3";
package cognyx.permission.v1;

service PermissionService {
  rpc MintCapability(MintCapabilityRequest) returns (common.CapabilityToken);
  rpc DelegateCapability(DelegateRequest) returns (common.CapabilityToken);
  rpc RevokeCapability(RevokeRequest) returns (google.protobuf.Empty);
  rpc ValidateCapability(ValidateRequest) returns (CapabilityValidation);
  rpc ListGrantedCapabilities(ListGrantedRequest) returns (ListGrantedResponse);
  rpc CheckOperationAllowed(CheckRequest) returns (CheckResponse);
  rpc RequestConsentPrompt(ConsentPromptRequest) returns (ConsentDecision);
  rpc GetCapabilityInfo(CapabilityInfoRequest) returns (CapabilityInfo);
  rpc WatchPermissionEvents(WatchRequest) returns (stream PermissionEvent);
}

message CapabilityValidation {
  bool valid = 1;
  optional common.Error failure_reason = 2;
  optional uint64 remaining_uses = 3;
  optional google.protobuf.Timestamp valid_until = 4;
  repeated string actual_operations = 5;   // Expanded operations
  repeated string actual_resources = 6;    // Expanded resources
}
```

---

## 12. Context API

```protobuf
syntax = "proto3";
package cognyx.context.v1;

message ContextPackage {
  common.Uuid package_id = 1;
  uint32 total_tokens = 2;
  uint32 budget_used_pct = 3;
  repeated ContextSection sections = 4;
  google.protobuf.Struct structured_state = 5;
}

message ContextSection {
  ContextCategory category = 1;
  uint32 token_count = 2;
  repeated ContextItem items = 3;
}

enum ContextCategory {
  SYSTEM_PROMPT = 0;
  RECENT_CONVERSATION = 1;
  EPISODIC_MEMORY = 2;
  SEMANTIC_FACTS = 3;
  WORKSPACE_FILES = 4;
  PROCEDURES = 5;
  TOOL_CAPABILITIES = 6;
  USER_PREFERENCES = 7;
}

message WorkingMemorySnapshot {
  repeated ConversationTurn recent_turns = 1;
  optional common.Uuid active_plan_id = 2;
  repeated ActiveTask active_tasks = 3;
  map<string, google.protobuf.Value> scratchpad = 4;
}

service ContextEngineService {
  rpc AssembleContext(AssembleRequest) returns (ContextPackage);
  rpc UpdateWorkingMemory(UpdateWMRequest) returns (google.protobuf.Empty);
  rpc GetWorkingMemory(GetWMRequest) returns (WorkingMemorySnapshot);
  rpc ClearWorkingMemory(ClearWMRequest) returns (google.protobuf.Empty);
  rpc IndexWorkspaceFiles(IndexFilesRequest) returns (IndexResult);
  rpc SearchWorkspaceContext(SearchContextRequest) returns (ContextHits);
}
```

---

## 13. Device API

```protobuf
syntax = "proto3";
package cognyx.device.v1;

message Device {
  common.Uuid device_id = 1;
  DeviceBus bus = 2;
  string vendor_id = 3;
  string product_id = 4;
  string vendor_name = 5;
  string product_name = 6;
  DeviceClass device_class = 7;
  DevicePowerState power_state = 8;
  repeated string driver_names = 9;
  string sysfs_path = 10;
}

enum DeviceBus { PCI = 0; USB = 1; BLUETOOTH = 2; I2C = 3; SPI = 4; PLATFORM = 5; VIRTUAL = 6; }
enum DeviceClass { GPU = 0; STORAGE = 1; NETWORK = 2; AUDIO = 3;
                   HID = 4; CAMERA = 5; BLUETOOTH_ADAPTER = 6; OTHER = 7; }
enum DevicePowerState { D0_ACTIVE = 0; D1 = 1; D2 = 2; D3_HOT = 3; D3_COLD = 4; }

message DeviceCapability {
  common.CapabilityToken token = 1;
  common.Uuid device_id = 2;
  repeated DeviceAccessMode allowed_modes = 3;
}
enum DeviceAccessMode { READ = 0; WRITE = 1; IOCTL = 2; MMAP = 3; PASSTHROUGH = 4; }

service DeviceManagerService {
  rpc ListDevices(ListDevicesRequest) returns (ListDevicesResponse);
  rpc GetDeviceInfo(GetDeviceRequest) returns (Device);
  rpc RequestDeviceAccess(AccessRequest) returns (DeviceCapability);
  rpc ReleaseDeviceAccess(ReleaseRequest) returns (google.protobuf.Empty);
  rpc ConfigureGpuPassthrough(GpuPassthroughRequest) returns (google.protobuf.Empty);
  rpc AuthorizeUsbDevice(UsbAuthRequest) returns (google.protobuf.Empty);
  rpc WatchDeviceEvents(WatchRequest) returns (stream DeviceEvent);
  rpc SetDevicePowerState(PowerStateRequest) returns (google.protobuf.Empty);
}
```

---

## 14. Window API

```protobuf
syntax = "proto3";
package cognyx.window.v1;

message Window {
  common.Uuid window_id = 1;
  string title = 2;
  optional common.Uuid workspace_id = 3;
  string app_id = 4;
  Rectangle geometry = 5;
  WindowState state = 6;
  WindowType type = 7;
  uint32 z_order = 8;
  optional common.Uuid parent_window_id = 9;
}

message Rectangle { int32 x = 1; int32 y = 2; int32 width = 3; int32 height = 4; }

enum WindowState {
  NORMAL = 0; MINIMIZED = 1; MAXIMIZED = 2; FULLSCREEN = 3; TILED = 4; FLOATING = 5;
}
enum WindowType {
  REGULAR = 0; DIALOG = 1; POPUP = 2; DOCK = 3; TOOLBAR = 4;
  NOTIFICATION = 5; SCREENSAVER = 6; DESKTOP = 7; OVERLAY = 8;
}

service WindowManagerService {
  rpc ListWindows(ListWindowsRequest) returns (ListWindowsResponse);
  rpc GetWindow(GetWindowRequest) returns (Window);
  rpc MoveResizeWindow(MoveResizeRequest) returns (Window);
  rpc SetWindowState(SetStateRequest) returns (Window);
  rpc CloseWindow(CloseRequest) returns (google.protobuf.Empty);
  rpc FocusWindow(FocusRequest) returns (google.protobuf.Empty);
  rpc TakeScreenshot(ScreenshotRequest) returns (ScreenshotResponse);
  rpc RegisterGlobalShortcut(ShortcutRequest) returns (ShortcutRegistration);
  rpc WatchWindowEvents(WatchRequest) returns (stream WindowEvent);
}
```

---

## 15. Application API

```protobuf
syntax = "proto3";
package cognyx.app.v1;

message InstalledApp {
  string app_id = 1;          // Reverse-DNS
  string display_name = 2;
  string version = 3;
  optional common.Uuid workspace_id = 4;  // null = global
  AppRuntimeType runtime = 5;
  repeated string declared_capabilities = 6;
  AppState state = 7;
}
enum AppRuntimeType { NATIVE = 0; CONTAINER = 1; VM = 2; COMPAT = 3; PLUGIN = 4; }
enum AppState { STOPPED = 0; STARTING = 1; RUNNING = 2; PAUSED = 3; ERROR = 4; }

message RunningInstance {
  common.Uuid instance_id = 1;
  string app_id = 2;
  optional common.Uuid workspace_id = 3;
  uint32 pid = 4;
  repeated common.Uuid window_ids = 5;
  google.protobuf.Timestamp started_at = 6;
}

service ApplicationRuntimeService {
  rpc InstallApp(InstallAppRequest) returns (InstalledApp);
  rpc UninstallApp(UninstallAppRequest) returns (google.protobuf.Empty);
  rpc LaunchApp(LaunchAppRequest) returns (RunningInstance);
  rpc TerminateApp(TerminateRequest) returns (google.protobuf.Empty);
  rpc ListInstalledApps(ListRequest) returns (ListInstalledResponse);
  rpc ListRunningInstances(ListInstancesRequest) returns (ListInstancesResponse);
  rpc GetAppInfo(AppInfoRequest) returns (InstalledApp);
  rpc WatchAppEvents(WatchRequest) returns (stream AppEvent);
}
```

---

## 16. Container API

```protobuf
syntax = "proto3";
package cognyx.container.v1;

message Container {
  common.Uuid container_id = 1;
  string image_reference = 2;
  optional string image_digest_sha256 = 3;
  optional common.Uuid workspace_id = 4;
  ContainerState state = 5;
  repeated string published_ports = 6;
  repeated Mount mounts = 7;
  repeated EnvironmentVar env = 8;
  google.protobuf.Timestamp created_at = 9;
  optional uint32 pid = 10;
}
enum ContainerState { CREATED = 0; RUNNING = 1; PAUSED = 2; STOPPED = 3; ERROR = 4; DELETED = 5; }

message Mount { string source = 1; string target = 2; MountType type = 3; bool read_only = 4; }
enum MountType { BIND = 0; VOLUME = 1; TMPFS = 2; }
message EnvironmentVar { string key = 1; string value = 2; }

service ContainerRuntimeService {
  rpc PullImage(PullImageRequest) returns (stream PullProgress);
  rpc CreateContainer(CreateContainerRequest) returns (Container);
  rpc StartContainer(StartContainerRequest) returns (google.protobuf.Empty);
  rpc StopContainer(StopContainerRequest) returns (google.protobuf.Empty);
  rpc PauseContainer(PauseRequest) returns (google.protobuf.Empty);
  rpc UnpauseContainer(UnpauseRequest) returns (google.protobuf.Empty);
  rpc DeleteContainer(DeleteRequest) returns (google.protobuf.Empty);
  rpc ListContainers(ListRequest) returns (ListContainersResponse);
  rpc GetContainerStatus(StatusRequest) returns (Container);
  rpc ExecInContainer(ExecRequest) returns (ExecHandle);
  rpc StreamContainerLogs(LogsRequest) returns (stream LogLine);
  rpc WatchContainerEvents(WatchRequest) returns (stream ContainerEvent);
}
```

---

## 17. VM API

```protobuf
syntax = "proto3";
package cognyx.vm.v1;

message VirtualMachine {
  common.Uuid vm_id = 1;
  string name = 2;
  GuestOSType os_type = 3;
  optional common.Uuid workspace_id = 4;
  VmState state = 5;
  uint32 vcpu_count = 6;
  uint64 memory_mb = 7;
  repeated DiskAttachment disks = 8;
  repeated NetworkInterface nics = 9;
  GpuAssignmentMode gpu_mode = 10;
  bool secure_boot_enabled = 11;
  bool tpm_enabled = 12;
}

enum GuestOSType { LINUX = 0; WINDOWS_10 = 1; WINDOWS_11 = 2; MACOS = 3; BSD = 4; OTHER = 5; }
enum VmState { STOPPED = 0; RUNNING = 1; PAUSED = 2; SAVED = 3; ERROR = 4; }
enum GpuAssignmentMode { NONE = 0; VIRTIO = 1; DRM_LEASE = 2; SRIOV = 3; FULL_PASSTHROUGH = 4; }

message DiskAttachment {
  common.Uuid disk_id = 1;
  uint64 size_bytes = 2;
  string path = 3;
  DiskBus bus = 4;
  bool read_only = 5;
}
enum DiskBus { VIRTIO = 0; SATA = 1; SCSI = 2; NVME = 3; IDE = 4; }

service VmManagerService {
  rpc CreateVm(CreateVmRequest) returns (VirtualMachine);
  rpc StartVm(StartVmRequest) returns (google.protobuf.Empty);
  rpc StopVm(StopVmRequest) returns (google.protobuf.Empty);
  rpc PauseVm(PauseVmRequest) returns (google.protobuf.Empty);
  rpc ResumeVm(ResumeVmRequest) returns (google.protobuf.Empty);
  rpc DeleteVm(DeleteVmRequest) returns (google.protobuf.Empty);
  rpc SnapshotVm(SnapshotRequest) returns (VmSnapshot);
  rpc RestoreVmSnapshot(RestoreRequest) returns (google.protobuf.Empty);
  rpc CloneVm(CloneVmRequest) returns (VirtualMachine);
  rpc ListVms(ListRequest) returns (ListVmsResponse);
  rpc GetVmStatus(StatusRequest) returns (VirtualMachine);
  rpc ConnectDisplay(DisplayRequest) returns (DisplayStreamInfo);
  rpc AttachDevice(AttachDeviceRequest) returns (google.protobuf.Empty);
  rpc DetachDevice(DetachDeviceRequest) returns (google.protobuf.Empty);
  rpc WatchVmEvents(WatchRequest) returns (stream VmEvent);
}
```

---

## 18. Search API

```protobuf
syntax = "proto3";
package cognyx.search.v1;

message SearchQuery {
  string text = 1;
  optional common.Uuid workspace_id = 2;
  repeated SearchScope scope = 3;  // Default = all scopes
  uint32 top_k = 4;                // Default = 20
  optional google.protobuf.Timestamp modified_after = 5;
  optional string file_type_filter = 6;
  optional float semantic_weight = 7;  // 0 = keyword only; 1 = semantic only (default 0.5)
}

enum SearchScope {
  FILE_CONTENT = 0;
  FILE_METADATA = 1;
  EPISODIC_MEMORY = 2;
  SEMANTIC_TRIPLES = 3;
  EMAILS = 4;
  NOTIFICATIONS = 5;
  APPS = 6;
  PLUGINS = 7;
  WORKSPACES = 8;
}

message SearchResult {
  oneof result {
    FileHit file = 1;
    MemoryHit memory = 2;
    NotificationHit notification = 3;
    AppHit app = 4;
    WorkspaceHit workspace = 5;
  }
  float relevance_score = 10;
  repeated string matched_keywords = 11;
  optional string highlight_snippet = 12;
}

message SearchResponse {
  repeated SearchResult results = 1;
  uint64 total_matches_estimated = 2;
  google.protobuf.Duration latency = 3;
}

service SearchService {
  rpc Search(SearchQuery) returns (SearchResponse);
  rpc SemanticSearch(SemanticQuery) returns (SearchResponse);
  rpc HybridSearch(HybridQuery) returns (SearchResponse);
  rpc IndexDocument(IndexDocumentRequest) returns (common.Uuid);
  rpc RemoveFromIndex(RemoveRequest) returns (google.protobuf.Empty);
  rpc GetSearchStats(StatsRequest) returns (SearchStats);
  rpc WatchIndexEvents(WatchRequest) returns (stream IndexEvent);
}
```
