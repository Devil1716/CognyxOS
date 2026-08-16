"""Filesystem read capability tool.

Implements a single "filesystem.read" capability following the capability
descriptor format from docs/contracts/capabilities-tools.md:

    {capability_id, version, risk_level, input_schema, output_schema,
     required_permissions, failure_modes, idempotency, audit_classification}

The tool reads a text file inside an allowed root directory and returns its
content. Invocation is audited through the existing EventBus from
cognyx_runtime.events using the standard ``org.cognyx.tool.executed`` event
type; every invocation produces an audit event, successful or not.

The required_permissions field only *declares* the permission requirement
described by the contract. No permission broker exists in this repository yet,
so nothing here enforces authorization at runtime.
"""

from pathlib import Path
from uuid import uuid4

from cognyx_runtime.events import Event, EventBus

# Inline JSON schemas for the tool's input and output. They are intentionally
# small so validation needs no third-party package; the capability descriptor
# references them by URI according to the JSON-schema reference convention.
INPUT_SCHEMA: dict[str, object] = {
    "type": "object",
    "properties": {"path": {"type": "string"}},
    "required": ["path"],
    "additionalProperties": False,
}

OUTPUT_SCHEMA: dict[str, object] = {
    "type": "object",
    "properties": {
        "path": {"type": "string"},
        "content": {"type": "string"},
    },
    "required": ["path", "content"],
    "additionalProperties": False,
}


class ReadToolError(RuntimeError):
    """Standard tool error carrying the capability failure mode."""

    def __init__(self, failure_mode: str, message: str) -> None:
        super().__init__(message)
        self.failure_mode = failure_mode
        self.message = message


class FilesystemReadTool:
    """Reads a text file that lives inside an allowed root directory.

    Dependencies (event_bus, publisher) are constructor-injected, mirroring
    BaseAgent; the tool cannot discover secrets or instantiate platform
    services.
    """

    capability_id = "org.cognyx.filesystem.read"
    tool_id = capability_id
    version = "1.0"
    risk_level = "low"
    input_schema = "https://schemas.cognyx.org/filesystem/v1/read-input.schema.json"
    output_schema = "https://schemas.cognyx.org/filesystem/v1/read-output.schema.json"
    required_permissions: tuple[str, ...] = ("filesystem.read",)
    failure_modes: tuple[str, ...] = (
        "validation_failed",
        "resource_unavailable",
        "internal",
    )
    idempotency = "supported"
    audit_classification = "restricted"

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
        }

    def _publish_event(
        self,
        event_type: str,
        payload: dict[str, object],
    ) -> None:
        """Publish a tool event when an event bus is configured.

        Mirrors BaseAgent._publish_event: the event bus is optional, and a
        publisher identity is required whenever a bus is present.
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
            raise ReadToolError(
                "validation_failed",
                "Request must be a JSON object.",
            )

        path = request.get("path")
        if not isinstance(path, str) or not path:
            raise ReadToolError(
                "validation_failed",
                "Request field 'path' must be a non-empty string.",
            )

        return {"path": path}

    def _resolve_within_root(self, path: str) -> Path:
        """Resolve a candidate path and reject anything outside the root.

        Relative paths are resolved against the allowed root rather than the
        process working directory, so a bare file name always stays inside the
        sandbox.
        """
        raw = Path(path).expanduser()
        candidate = (
            (self.allowed_root / raw).resolve()
            if not raw.is_absolute()
            else raw.resolve()
        )
        if not candidate.is_relative_to(self.allowed_root):
            raise ReadToolError(
                "validation_failed",
                "Path is outside the allowed root directory.",
            )
        return candidate

    @staticmethod
    def _read_text(path: Path) -> str:
        """Read the file's UTF-8 text content with standard failure modes."""
        try:
            return path.read_text(encoding="utf-8")
        except FileNotFoundError:
            raise ReadToolError(
                "resource_unavailable",
                f"File not found: {path}",
            ) from None
        except IsADirectoryError:
            raise ReadToolError(
                "resource_unavailable",
                f"Path is not a file: {path}",
            ) from None
        except OSError as exc:
            raise ReadToolError(
                "resource_unavailable",
                f"Could not read file: {path} ({exc})",
            ) from None

    def execute(self, request: dict[str, object]) -> dict[str, object]:
        """Validate, execute, and audit one filesystem.read invocation.

        Emits ``org.cognyx.tool.executed`` for every invocation, whether it
        completes or fails. File content is never placed on the event bus.
        """
        invocation_id = str(uuid4())

        try:
            validated = self.validate_request(request)
            resolved = self._resolve_within_root(validated["path"])
            content = self._read_text(resolved)
        except ReadToolError as exc:
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

        return {"path": str(resolved), "content": content}