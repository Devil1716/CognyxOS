"""Thread-safe in-memory memory store (minimal slice).

There is no dedicated memory contract in this repository. This slice is a
minimal interpretation of the single documented memory event,
org.cognyx.memory.updated (docs/contracts/events.md), which carries
memory_id, operation, and revision. It adds no contract rules of its own.

Deliberately NOT implemented: persistence, agent namespacing/scoping, query or
search, and any behavior beyond what the memory event documents. Revisions are
per-memory_id, start at 1 on the first write, and never reset (a delete keeps
the counter counting).
"""

from threading import RLock

from cognyx_runtime.events import Event, EventBus


class MemoryError(RuntimeError):
    """Raised when a memory operation fails (mirrors ReadToolError style)."""

    def __init__(self, failure_mode: str, message: str) -> None:
        super().__init__(message)
        self.failure_mode = failure_mode
        self.message = message


class MemoryStore:
    """RLock-guarded in-memory key-value store with revisioned events."""

    def __init__(
        self,
        *,
        event_bus: EventBus | None = None,
        publisher: str | None = None,
    ) -> None:
        if event_bus is not None and not publisher:
            raise ValueError(
                "publisher is required when event_bus is provided."
            )

        self.event_bus = event_bus
        self.publisher = publisher

        self._data: dict[str, object] = {}
        self._revisions: dict[str, int] = {}
        self._lock = RLock()

    def _publish_event(
        self,
        memory_id: str,
        operation: str,
        revision: int,
    ) -> None:
        if self.event_bus is None:
            return

        if self.publisher is None:
            raise RuntimeError("Memory event publisher is not configured.")

        self.event_bus.publish(
            Event(
                event_type="org.cognyx.memory.updated",
                payload={
                    "memory_id": memory_id,
                    "operation": operation,
                    "revision": revision,
                },
                publisher=self.publisher,
                aggregate_id=memory_id,
            )
        )

    def set(self, memory_id: str, value: object) -> None:
        """Store a value, starting/advancing that memory_id's revision.

        Publishes org.cognyx.memory.updated with operation="write".
        """
        with self._lock:
            revision = self._revisions.get(memory_id, 0) + 1
            self._revisions[memory_id] = revision
            self._data[memory_id] = value

        self._publish_event(memory_id, "write", revision)

    def get(self, memory_id: str) -> object:
        """Return the stored value.

        A missing key raises MemoryError(resource_unavailable); no event is
        published.
        """
        with self._lock:
            if memory_id not in self._data:
                raise MemoryError(
                    "resource_unavailable",
                    f"Memory not found: {memory_id}",
                )
            return self._data[memory_id]

    def delete(self, memory_id: str) -> None:
        """Delete a value if present and publish a delete event.

        Publishes org.cognyx.memory.updated with operation="delete" and the
        next revision (the per-memory_id counter keeps counting and never
        resets). Deleting a missing key is a silent no-op: no event, no
        exception.
        """
        with self._lock:
            if memory_id not in self._data:
                return
            del self._data[memory_id]
            revision = self._revisions.get(memory_id, 0) + 1
            self._revisions[memory_id] = revision

        self._publish_event(memory_id, "delete", revision)