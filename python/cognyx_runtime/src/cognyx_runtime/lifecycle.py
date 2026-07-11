"""Serialized runtime lifecycle coordinator."""

from dataclasses import dataclass
from datetime import UTC, datetime
from enum import StrEnum
from threading import RLock

from .errors import LifecycleError


class RuntimeState(StrEnum):
    CREATED = "Created"
    INITIALIZING = "Initializing"
    STARTING = "Starting"
    RUNNING = "Running"
    PAUSED = "Paused"
    DEGRADED = "Degraded"
    RECOVERING = "Recovering"
    STOPPING = "Stopping"
    STOPPED = "Stopped"
    FAILED = "Failed"


_TRANSITIONS = {
    RuntimeState.CREATED: {RuntimeState.INITIALIZING, RuntimeState.FAILED},
    RuntimeState.INITIALIZING: {RuntimeState.STARTING, RuntimeState.FAILED},
    RuntimeState.STARTING: {
        RuntimeState.RUNNING,
        RuntimeState.DEGRADED,
        RuntimeState.FAILED,
        RuntimeState.STOPPING,
    },
    RuntimeState.RUNNING: {
        RuntimeState.PAUSED,
        RuntimeState.DEGRADED,
        RuntimeState.STOPPING,
        RuntimeState.FAILED,
    },
    RuntimeState.PAUSED: {RuntimeState.RUNNING, RuntimeState.STOPPING, RuntimeState.RECOVERING},
    RuntimeState.DEGRADED: {
        RuntimeState.RUNNING,
        RuntimeState.RECOVERING,
        RuntimeState.STOPPING,
        RuntimeState.FAILED,
    },
    RuntimeState.RECOVERING: {
        RuntimeState.RUNNING,
        RuntimeState.DEGRADED,
        RuntimeState.FAILED,
        RuntimeState.STOPPING,
    },
    RuntimeState.STOPPING: {RuntimeState.STOPPED, RuntimeState.FAILED},
    RuntimeState.STOPPED: {RuntimeState.CREATED},
    RuntimeState.FAILED: {RuntimeState.RECOVERING, RuntimeState.STOPPING, RuntimeState.STOPPED},
}


@dataclass(frozen=True, slots=True)
class LifecycleRecord:
    previous: RuntimeState
    current: RuntimeState
    reason: str
    correlation_id: str
    timestamp: str


class LifecycleCoordinator:
    def __init__(self) -> None:
        self.state = RuntimeState.CREATED
        self.history: list[LifecycleRecord] = []
        self._lock = RLock()

    def transition(self, target: RuntimeState, reason: str, correlation_id: str) -> LifecycleRecord:
        with self._lock:
            if target not in _TRANSITIONS[self.state]:
                raise LifecycleError(f"{self.state} cannot transition to {target}")
            record = LifecycleRecord(
                self.state, target, reason, correlation_id, datetime.now(UTC).isoformat()
            )
            self.state = target
            self.history.append(record)
            return record
