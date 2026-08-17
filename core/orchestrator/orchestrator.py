"""Orchestrator (first slice): single-adapter capability routing.

CONTRACT GAP: there is currently no formal orchestrator contract in this
repository - orchestration is only mentioned as a future concept and is not
specified in docs/contracts/ or cognyx-os/. This module is therefore a
minimal interpretation of the existing architecture (tasks declare
capabilities; adapters are registered per capability), NOT a verified
orchestration specification.

The Orchestrator's ONLY responsibility in this slice is answering the
question: "Can ONE registered CapabilityAdapter handle every capability
required by this entire TaskGraph?"

It deliberately does NOT:
- perform per-task routing of tasks to different adapters,
- coordinate multiple agents,
- execute or schedule tasks (execution belongs to TaskExecutor ->
  CapabilityAdapter),
- merge adapters, or
- make planning or supervision decisions.
"""

from core.executor.capability_adapter import CapabilityAdapter
from core.planner.task_graph import TaskGraph


class OrchestratorError(ValueError):
    """Raised when a graph cannot be routed to a single adapter."""


class Orchestrator:
    """Registers adapters by capability and routes whole graphs to one.

    The adapter mapping supplied at construction IS the registration
    mechanism: multiple capability strings may map to the SAME CapabilityAdapter
    instance, and that adapter is then considered able to satisfy every
    capability registered to it. No additional capability semantics exist.
    """

    def __init__(self, adapters: dict[str, CapabilityAdapter]) -> None:
        self._adapters = dict(adapters)

    def route(self, graph: TaskGraph) -> CapabilityAdapter:
        """Return the one adapter able to satisfy this whole graph.

        Collects the union of every capability declared by the graph's tasks,
        then resolves each capability to its registered adapter instance. The
        graph is never modified and no task is executed.

        Raises OrchestratorError when: the graph is empty, its tasks declare
        no capabilities at all, any required capability has no registered
        adapter, or the required capabilities resolve to more than one
        distinct adapter instance.
        """
        tasks = graph.tasks
        if not tasks:
            raise OrchestratorError(
                "Cannot route an empty task graph: no tasks are registered."
            )

        required: set[str] = set()
        for task in tasks:
            required.update(task.capabilities)

        if not required:
            raise OrchestratorError(
                "Cannot route a graph whose tasks declare no capabilities."
            )

        resolved: set[CapabilityAdapter] = set()
        for capability in sorted(required):
            adapter = self._adapters.get(capability)
            if adapter is None:
                raise OrchestratorError(
                    f"No adapter registered for required capability "
                    f"'{capability}'."
                )
            resolved.add(adapter)

        if len(resolved) > 1:
            raise OrchestratorError(
                "Graph requires capabilities that resolve to multiple "
                "distinct adapters and cannot be handled by a single adapter."
            )

        return next(iter(resolved))