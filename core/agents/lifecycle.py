"""Agent lifecycle coordination."""

from collections.abc import Callable
from dataclasses import dataclass
from datetime import UTC, datetime
from enum import StrEnum
from threading import RLock
from typing import Protocol


class AgentLifecycleError(ValueError):
    """Raised when an Agent lifecycle operation is invalid."""


class AgentState(StrEnum):
    CREATED = "Created"
    INITIALIZING = "Initializing"
    IDLE = "Idle"
    PLANNING = "Planning"
    WAITING = "Waiting"
    EXECUTING = "Executing"
    OBSERVING = "Observing"
    REASONING = "Reasoning"
    PAUSED = "Paused"
    RECOVERING = "Recovering"
    COMPLETED = "Completed"
    FAILED = "Failed"
    SHUTDOWN = "Shutdown"


@dataclass(frozen=True, slots=True)
class AgentLifecycleRecord:
    """Immutable record of one Agent lifecycle transition."""

    previous: AgentState
    current: AgentState
    reason: str
    correlation_id: str
    timestamp: str


class CheckpointWriter(Protocol):
    """Persistence boundary used by the lifecycle coordinator."""

    def __call__(self, record: AgentLifecycleRecord) -> None:
        """Persist a lifecycle checkpoint."""


_TRANSITIONS: dict[AgentState, set[AgentState]] = {
    AgentState.CREATED: {
        AgentState.INITIALIZING,
    },
    AgentState.INITIALIZING: {
        AgentState.IDLE,
        AgentState.FAILED,
    },
    AgentState.IDLE: {
        AgentState.PLANNING,
        AgentState.PAUSED,
        AgentState.COMPLETED,
    },
    AgentState.PLANNING: {
        AgentState.REASONING,
        AgentState.WAITING,
        AgentState.PAUSED,
    },
    AgentState.WAITING: {
        AgentState.PLANNING,
    },
    AgentState.EXECUTING: {
        AgentState.OBSERVING,
        AgentState.WAITING,
        AgentState.PAUSED,
        AgentState.FAILED,
    },
    AgentState.OBSERVING: {
        AgentState.IDLE,
    },
    AgentState.REASONING: {
        AgentState.EXECUTING,
    },
    AgentState.PAUSED: {
        AgentState.RECOVERING,
    },
    AgentState.RECOVERING: {
        AgentState.IDLE,
        AgentState.FAILED,
    },
    AgentState.COMPLETED: {
        AgentState.SHUTDOWN,
    },
    AgentState.FAILED: {
        AgentState.SHUTDOWN,
    },
    AgentState.SHUTDOWN: set(),
}


class AgentLifecycleCoordinator:
    """Own and validate Agent lifecycle transitions."""

    def __init__(
        self,
        checkpoint_writer: CheckpointWriter | None = None,
    ) -> None:
        self.state = AgentState.CREATED
        self.history: list[AgentLifecycleRecord] = []
        self._checkpoint_writer = checkpoint_writer
        self._lock = RLock()

    def transition(
        self,
        target: AgentState,
        reason: str,
        correlation_id: str,
    ) -> AgentLifecycleRecord:
        """Validate, checkpoint, and apply a lifecycle transition."""
        if not reason:
            raise AgentLifecycleError("Transition reason is required.")

        if not correlation_id:
            raise AgentLifecycleError("Correlation ID is required.")

        with self._lock:
            if target not in _TRANSITIONS[self.state]:
                raise AgentLifecycleError(
                    f"{self.state} cannot transition to {target}"
                )

            record = AgentLifecycleRecord(
                previous=self.state,
                current=target,
                reason=reason,
                correlation_id=correlation_id,
                timestamp=datetime.now(UTC).isoformat(),
            )

            # The checkpoint must succeed before the in-memory state changes.
            # This prevents the coordinator from reporting a transition that
            # failed at its persistence boundary.
            if self._checkpoint_writer is not None:
                self._checkpoint_writer(record)

            self.state = target
            self.history.append(record)

            return record

    def cancel(
        self,
        reason: str,
        correlation_id: str,
        compensation: Callable[[], None] | None = None,
        *,
        shutdown: bool = False,
    ) -> AgentLifecycleRecord:
        """Cancel a non-terminal Agent after optional compensation.

        Cancellation is a lifecycle operation rather than a normal state
        transition. A cancelled Agent enters Paused by default or Shutdown
        when explicitly requested.
        """
        with self._lock:
            if self.state in {
                AgentState.COMPLETED,
                AgentState.FAILED,
                AgentState.SHUTDOWN,
            }:
                raise AgentLifecycleError(
                    f"Cannot cancel terminal state {self.state}."
                )

            if compensation is not None:
                compensation()

            target = (
                AgentState.SHUTDOWN
                if shutdown
                else AgentState.PAUSED
            )

            record = AgentLifecycleRecord(
                previous=self.state,
                current=target,
                reason=reason,
                correlation_id=correlation_id,
                timestamp=datetime.now(UTC).isoformat(),
            )

            if self._checkpoint_writer is not None:
                self._checkpoint_writer(record)

            self.state = target
            self.history.append(record)

            return record