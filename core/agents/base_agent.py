"""Base Agent lifecycle and identity."""

from dataclasses import dataclass
from uuid import uuid4

from cognyx_runtime.events import Event, EventBus

from core.executor.capability_adapter import CapabilityAdapter
from core.executor.task_executor import TaskExecutor
from core.memory.memory_store import MemoryError, MemoryStore
from core.planner.task_graph import TaskGraph, TaskState

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
        memory_store: MemoryStore | None = None,
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
        self.memory_store = memory_store

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

    def run_task_graph(
        self,
        graph: TaskGraph,
        adapter: CapabilityAdapter,
        skip_completed_from_history: bool = False,
    ) -> None:
        """Drive planning/execution over a task graph, then finish the agent.

        Uses only this agent's existing lifecycle methods. It plans, reasons,
        executes, runs ``TaskExecutor(graph, adapter).run_all()``, then ends on
        COMPLETED when every task succeeded or on FAILED when any task failed
        (the failure reason names the failed task IDs).

        Lifecycle note: fail() is only reachable from the initializing,
        executing, or recovering states - never from observing or idle. So the
        failure branch must call fail() while the agent is still in Executing,
        and only the success branch advances through observation to Completed.
        This mirrors, rather than alters, the existing transition rules.
        """
        correlation_id = self.run_id

        self.start_planning(correlation_id)
        self.begin_reasoning(correlation_id)
        self.begin_execution(correlation_id)

        # Record execution order at the capability boundary without modifying
        # the adapter: TaskExecutor calls a recorder that delegates to it.
        execution: list[tuple[str, bool]] = []

        def recorder(task):
            outcome = adapter(task)
            execution.append((task.task_id, outcome))
            return outcome

        if skip_completed_from_history:
            previous = self.recall_previous_run()
            if previous:
                previously_completed = set(previous.get("completed_task_ids", []))
                for task in graph.tasks:
                    if task.task_id in previously_completed:
                        graph.transition(task.task_id, TaskState.COMPLETED)

        TaskExecutor(graph, recorder).run_all()

        failed = [
            task.task_id for task in graph.tasks if task.state == TaskState.FAILED
        ]
        if failed:
            self.fail(
                correlation_id,
                reason="Tasks failed: " + ", ".join(failed),
            )
            return

        self.begin_observation(correlation_id)
        self.return_to_idle(correlation_id)

        if self.memory_store is not None:
            # Build completed_ids from the graph's final state, not the
            # execution list: tasks pre-completed via resume never pass
            # through the recorder, but both resumed and newly-executed tasks
            # end up COMPLETED in the graph, so history is preserved across
            # chained resume runs.
            completed_ids = [
                task.task_id
                for task in graph.tasks
                if task.state == TaskState.COMPLETED
            ]
            summary = {
                "completed_task_ids": completed_ids,
                "outcome": "completed",
            }
            self.memory_store.set(self.run_id, summary)
            # The agent_id key always holds the most recent successful run's
            # summary (overwritten, never a growing history); run_id stays
            # per-run so each individual run keeps its own record too.
            self.memory_store.set(self.agent_id, summary)

        self.complete(correlation_id)

    def recall_previous_run(self) -> dict | None:
        """Return this agent's most recent successful run summary.

        Returns None when no memory store is provided, or when nothing has
        been recorded under self.agent_id yet (a missing key surfaces as
        MemoryError and is treated as "not found" only; any other exception
        propagates).

        By default this is only available to callers and does not influence
        decisions; run_task_graph consults it only when the caller opts in
        with skip_completed_from_history=True. Automatic, decision-driving
        planning from memory remains future work.
        """
        if self.memory_store is None:
            return None

        try:
            value = self.memory_store.get(self.agent_id)
        except MemoryError:
            return None

        return value if isinstance(value, dict) else None