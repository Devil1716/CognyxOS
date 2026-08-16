"""Base Agent lifecycle and identity."""

from dataclasses import dataclass
from uuid import uuid4

from cognyx_runtime.events import Event, EventBus

from .lifecycle import AgentLifecycleCoordinator, AgentState


@dataclass(slots=True)
class AgentIdentity:
    """Identity associated with an Agent run."""

    agent_id: str
    run_id: str
    goal_id: str


class BaseAgent:
    """Minimal lifecycle-aware Agent foundation.

    The BaseAgent owns Agent identity and delegates all lifecycle state
    changes to AgentLifecycleCoordinator.
    """
    def _publish_event(
        self,
        event_type: str,
        payload: dict[str, object],
    ) -> None:
        """Publish an Agent lifecycle event when an event bus is configured."""
        if self.event_bus is None:
            return

        if self.publisher is None:
            raise RuntimeError("Agent event publisher is not configured.")

        self.event_bus.publish(
            Event(
                event_type=event_type,
                payload=payload,
                publisher=self.publisher,
                aggregate_id=self.agent_id,
            )
        )
    def __init__(
        self,
        goal_id: str,
        *,
        agent_id: str | None = None,
        run_id: str | None = None,
        lifecycle: AgentLifecycleCoordinator | None = None,
        event_bus: EventBus | None = None,
        publisher: str | None = None,
    ) -> None:
        if not goal_id:
            raise ValueError("goal_id is required.")

        self.identity = AgentIdentity(
            agent_id=agent_id or str(uuid4()),
            run_id=run_id or str(uuid4()),
            goal_id=goal_id,
        )

        self.lifecycle = lifecycle or AgentLifecycleCoordinator()
        if event_bus is not None and not publisher:
            raise ValueError(
                "publisher is required when event_bus is provided."
        )

        self.event_bus = event_bus
        self.publisher = publisher

    @property
    def agent_id(self) -> str:
        return self.identity.agent_id

    @property
    def run_id(self) -> str:
        return self.identity.run_id

    @property
    def goal_id(self) -> str:
        return self.identity.goal_id

    @property
    def state(self) -> AgentState:
        return self.lifecycle.state

    def initialize(self, correlation_id: str) -> None:
        """Initialize the Agent and make it idle."""
        self.lifecycle.transition(
            AgentState.INITIALIZING,
            reason="Agent initialization started.",
            correlation_id=correlation_id,
        )

        self.lifecycle.transition(
            AgentState.IDLE,
            reason="Agent initialization completed.",
            correlation_id=correlation_id,
        )

        self._publish_event(
            "org.cognyx.agent.started",
            {
                "agent_id": self.agent_id,
                "run_id": self.run_id,
                "goal_id": self.goal_id,
            },
        )

    def start_planning(self, correlation_id: str) -> None:
        """Move an idle Agent into planning."""
        self.lifecycle.transition(
            AgentState.PLANNING,
            reason="Agent planning started.",
            correlation_id=correlation_id,
        )

    def begin_reasoning(self, correlation_id: str) -> None:
        """Move a planning Agent into reasoning."""
        self.lifecycle.transition(
            AgentState.REASONING,
            reason="Agent reasoning started.",
            correlation_id=correlation_id,
        )

    def begin_execution(self, correlation_id: str) -> None:
        """Move a reasoning Agent into execution."""
        self.lifecycle.transition(
            AgentState.EXECUTING,
            reason="Agent execution started.",
            correlation_id=correlation_id,
        )

    def begin_observation(self, correlation_id: str) -> None:
        """Move an executing Agent into observation."""
        self.lifecycle.transition(
            AgentState.OBSERVING,
            reason="Agent observation started.",
            correlation_id=correlation_id,
        )

    def return_to_idle(self, correlation_id: str) -> None:
        """Return an observed Agent to idle."""
        self.lifecycle.transition(
            AgentState.IDLE,
            reason="Agent observation completed.",
            correlation_id=correlation_id,
        )

    def complete(self, correlation_id: str) -> None:
        """Complete an idle Agent."""
        self.lifecycle.transition(
            AgentState.COMPLETED,
            reason="Agent completed successfully.",
            correlation_id=correlation_id,
        )

        self._publish_event(
            "org.cognyx.agent.finished",
            {
                "agent_id": self.agent_id,
                "run_id": self.run_id,
                "outcome": "completed",
            },
        )

    def fail(
        self,
        correlation_id: str,
        reason: str = "Agent failed.",
    ) -> None:
        """Move the Agent into the terminal failure state."""
        self.lifecycle.transition(
            AgentState.FAILED,
            reason=reason,
            correlation_id=correlation_id,
        )

        self._publish_event(
            "org.cognyx.agent.finished",
            {
                "agent_id": self.agent_id,
                "run_id": self.run_id,
                "outcome": "failed",
            },
        )

    def recover(self, correlation_id: str) -> None:
        """Recover a paused Agent."""
        self.lifecycle.transition(
            AgentState.RECOVERING,
            reason="Agent recovery started.",
            correlation_id=correlation_id,
        )

        self.lifecycle.transition(
            AgentState.IDLE,
            reason="Agent recovery completed.",
            correlation_id=correlation_id,
        )

    def shutdown(self, correlation_id: str) -> None:
        """Shut down a completed or failed Agent."""
        self.lifecycle.transition(
            AgentState.SHUTDOWN,
            reason="Agent shutdown requested.",
            correlation_id=correlation_id,
        )