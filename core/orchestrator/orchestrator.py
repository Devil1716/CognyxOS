"""Orchestrator (second, minimal slice): per-task adapter dispatch.

CONTRACT GAP: there is still no formal orchestrator contract in this
repository - orchestration is only mentioned as a future concept and is not
specified in docs/contracts/ or cognyx-os/. This module is therefore a
minimal interpretation of the existing architecture (tasks declare
capabilities; adapters are registered per capability), NOT a verified
orchestration specification.

FIRST slice: Orchestrator.route() answers "can ONE registered adapter handle
every capability required by this entire TaskGraph?".

SECOND slice (this one): Orchestrator.dispatch() resolves a single task to
the one adapter registered for its capabilities, and Orchestrator.run_graph()
executes an entire graph by letting dispatch() pick the adapter for every
task just-in-time as TaskExecutor asks for work - all within ONE graph,
without pre-wiring tasks to adapters.

It deliberately does NOT implement: multiple agents, parallel execution,
agent-to-agent routing, supervision, planning, deciding which agent owns a
goal, or any behavior beyond per-task dispatch and single-graph execution.
"""

from core.executor.capability_adapter import CapabilityAdapter
from core.executor.task_executor import TaskExecutor
from core.planner.task_graph import Task, TaskGraph


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

    def dispatch(self, task: Task) -> CapabilityAdapter:
        """Return the single adapter registered for this task's capabilities.

        Raises OrchestratorError when the task declares no capabilities, when
        a required capability has no registered adapter (naming it), or when
        the task's capabilities resolve to more than one distinct adapter.
        Uses the same capability -> adapter mapping as route(); no new
        registration API exists.
        """
        required = set(task.capabilities)
        if not required:
            raise OrchestratorError(
                f"Task '{task.task_id}' declares no capabilities."
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
                f"Task '{task.task_id}' requires capabilities that resolve to "
                "multiple distinct adapters and cannot be handled by a single "
                "adapter."
            )

        return next(iter(resolved))

    def run_graph(self, graph: TaskGraph) -> None:
        """Execute the graph through the existing TaskExecutor.

        Adapter choice happens just-in-time per task: TaskExecutor receives a
        single run_task callable that dispatches and invokes the task's
        adapter. OrchestratorError is NOT caught here - an unresolvable task's
        dispatch error propagates into TaskExecutor, which treats an exception
        from run_task exactly like a False result and marks that task FAILED
        (this integrates with TaskExecutor's existing exception handling).
        """
        def run_task(task: Task) -> bool:
            return self.dispatch(task)(task)

        TaskExecutor(graph, run_task).run_all()