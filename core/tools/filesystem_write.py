"""Filesystem write capability tool.

Implements a single "filesystem.write" capability following the capability
descriptor format from docs/contracts/capabilities-tools.md:

    {capability_id, version, risk_level, input_schema, output_schema,
     required_permissions, failure_modes, idempotency, audit_classification}

The tool writes UTF-8 text to a file inside an allowed root directory and
returns the number of bytes written. Invocation is audited through the
existing EventBus using the standard ``org.cognyx.tool.executed`` event type;
every invocation produces an audit event, successful or not, and file content
is NEVER placed on the event bus.

Safety boundary (same as FilesystemReadTool, plus one extra guard): relative
paths resolve against the allowed root; absolute paths are checked for
containment; symlinks are resolved first. Additionally, the resolved PARENT
directory must also stay inside the root - this refuses writes that would
land outside even when the target leaf does not exist yet (there is nothing
to resolve at the leaf's own position).

Behavior decisions for this slice:
- Directories are NOT auto-created. Writing into a non-existent parent
  directory fails with failure_mode="internal" (an OSError during the write);
  callers must ensure the target directory already exists.
- ``idempotency`` is declared "not_supported". The contract (capability
  descriptor schema) enumerates required/supported/not_supported but does not
  define per-operation prose semantics. Under the natural reading where
  "supported" means repeat/replay-safe, an overwriting write is NOT
  repeat-safe: writing the same path twice overwrites the previous content
  with no idempotency guard. This slice therefore truthfully declares
  not_supported.

The required_permissions field only *declares* the permission requirement
described by the contract. No permission broker exists in this repository
yet, so nothing here enforces authorization at runtime.
"""

from pathlib import Path
from uuid import uuid4

from cognyx_runtime.events import Event, EventBus

# Inline JSON schemas for the tool's input and output. They are intentionally
# small so validation needs no third-party package; the capability descriptor
# references them by URI according to the JSON-schema reference convention.
INPUT_SCHEMA: dict[str, object] = {
    "type": "object",
    "properties": {
        "path": {"type": "string"},
        "content": {"type": "string"},
    },
    "required": ["path", "content"],
    "additionalProperties": False,
}

OUTPUT_SCHEMA: dict[str, object] = {
    "type": "object",
    "properties": {
        "path": {"type": "string"},
        "bytes_written": {"type": "integer"},
    },
    "required": ["path", "bytes_written"],
    "additionalProperties": False,
}


class WriteToolError(RuntimeError):
    """Standard tool error carrying the capability failure mode."""

    def __init__(self, failure_mode: str, message: str) -> None:
        super().__init__(message)
        self.failure_mode = failure_mode
        self.message = message


class FilesystemWriteTool:
    """Writes a UTF-8 text file that lives inside an allowed root directory.

    Dependencies (event_bus, publisher) are constructor-injected, mirroring
    FilesystemReadTool; the tool cannot discover secrets or instantiate
    platform services.
    """

    capability_id = "org.cognyx.filesystem.write"
    tool_id = capability_id
    version = "1.0"
    # Contract default risk for filesystem writes is "confirmation".
    risk_level = "confirmation"
    input_schema = "https://schemas.cognyx.org/filesystem/v1/write-input.schema.json"
    output_schema = "https://schemas.cognyx.org/filesystem/v1/write-output.schema.json"
    required_permissions: tuple[str, ...] = ("filesystem.write",)
    failure_modes: tuple[str, ...] = (
        "validation_failed",
        "internal",
    )
    idempotency = "not_supported"
    audit_classification = "restricted"
    # Write overwrites files, so it is destructive under the contract: an
    # explicit rollback declaration is required and none is implemented.
    rollback = "unsupported"

    def __init__(
        self,
        allowed_root: str | Path,
        *,
        event_bus: EventBus | None = None,
        publisher: str | None = None,
    ) -> None:
        if not allowed_root:
            raise ValueError("allowed_root is required.")

        self.allowed_root = Path(allowed_root).expanduser().resolve()
        if event_bus is not None and not publisher:
            raise ValueError(
                "publisher is required when event_bus is provided."
            )

        self.event_bus = event_bus
        self.publisher = publisher

    @property
    def descriptor(self) -> dict[str, object]:
        """The capability descriptor declared by the contract."""
        return {
            "capability_id": self.capability_id,
            "version": self.version,
            "risk_level": self.risk_level,
            "input_schema": self.input_schema,
            "output_schema": self.output_schema,
            "required_permissions": list(self.required_permissions),
            "failure_modes": list(self.failure_modes),
            "idempotency": self.idempotency,
            "audit_classification": self.audit_classification,
            # Destructive operations must declare rollback explicitly.
            "rollback": self.rollback,
        }

    def _publish_event(
        self,
        event_type: str,
        payload: dict[str, object],
    ) -> None:
        """Publish a tool event when an event bus is configured.

        Mirrors FilesystemReadTool._publish_event: the event bus is optional,
        and a publisher identity is required whenever a bus is present.
        """
        if self.event_bus is None:
            return

        if self.publisher is None:
            raise RuntimeError("Tool event publisher is not configured.")

        self.event_bus.publish(
            Event(
                event_type=event_type,
                payload=payload,
                publisher=self.publisher,
                aggregate_id=self.tool_id,
            )
        )

    def validate_request(self, request: dict[str, object]) -> dict[str, str]:
        """Validate the input request against the declared input schema."""
        if not isinstance(request, dict):
            raise WriteToolError(
                "validation_failed",
                "Request must be a JSON object.",
            )

        path = request.get("path")
        if not isinstance(path, str) or not path:
            raise WriteToolError(
                "validation_failed",
                "Request field 'path' must be a non-empty string.",
            )

        content = request.get("content")
        if not isinstance(content, str):
            raise WriteToolError(
                "validation_failed",
                "Request field 'content' must be a string.",
            )

        return {"path": path, "content": content}

    def _resolve_within_root(self, path: str) -> Path:
        """Resolve a write target and reject anything outside the root.

        Relative paths are resolved against the allowed root rather than the
        process working directory. Symlinks are resolved first, and BOTH the
        resolved target and its resolved PARENT must stay inside the root -
        this refuses escapes even when the leaf itself does not exist yet, so
        nothing is written or created outside the allowed root.
        """
        raw = Path(path).expanduser()
        candidate = (
            (self.allowed_root / raw).resolve()
            if not raw.is_absolute()
            else raw.resolve()
        )
        if (
            not candidate.is_relative_to(self.allowed_root)
            or not candidate.parent.is_relative_to(self.allowed_root)
        ):
            raise WriteToolError(
                "validation_failed",
                "Path is outside the allowed root directory.",
            )
        return candidate

    @staticmethod
    def _write_text(path: Path, content: str) -> int:
        """Write UTF-8 text and return the number of bytes written.

        Directories are never auto-created: writing into a non-existent
        parent directory surfaces as an OSError and becomes failure_mode
        "internal".
        """
        try:
            return path.write_bytes(content.encode("utf-8"))
        except OSError as exc:
            raise WriteToolError(
                "internal",
                f"Could not write file: {path} ({exc})",
            ) from None

    def execute(self, request: dict[str, object]) -> dict[str, object]:
        """Validate, execute, and audit one filesystem.write invocation.

        Emits ``org.cognyx.tool.executed`` for every invocation, whether it
        completes or fails. File content is never placed on the event bus.
        """
        invocation_id = str(uuid4())

        try:
            validated = self.validate_request(request)
            resolved = self._resolve_within_root(validated["path"])
            bytes_written = self._write_text(resolved, validated["content"])
        except WriteToolError as exc:
            self._publish_event(
                "org.cognyx.tool.executed",
                {
                    "tool_id": self.tool_id,
                    "invocation_id": invocation_id,
                    "outcome": "failed",
                    "failure_mode": exc.failure_mode,
                },
            )
            raise

        self._publish_event(
            "org.cognyx.tool.executed",
            {
                "tool_id": self.tool_id,
                "invocation_id": invocation_id,
                "outcome": "completed",
            },
        )

        return {"path": str(resolved), "bytes_written": bytes_written}