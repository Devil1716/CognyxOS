"""Sequential task executor (first slice).

Implements the scheduler/execution step of the task graph contract in its
minimal form: repeatedly ask the TaskGraph "what is ready?" ("the scheduler
runs only ready nodes"), move each ready node through
pending -> ready -> running, then call an injected run_task callable to
decide completion or failure.

Deliberately NOT included in this slice: parallelism, resource constraints,
retries, timeouts, verification, rollback, cancellation, and real tool/model
invocation - run_task is an injected fake that stands in for "actually call a
tool or model", which is wired up in a later slice.

No events are emitted here. org.cognyx.task.created is a planner-side event
(this executor consumes a pre-built graph), and org.cognyx.task.completed
requires a result_ref that a bool-returning run_task does not produce.
"""

from collections.abc import Callable

from core.planner.task_graph import Task, TaskGraph, TaskState


class TaskExecutor:
    """Runs ready task nodes to completion or failure, one at a time."""

    def __init__(
        self,
        graph: TaskGraph,
        run_task: Callable[[Task], bool],
    ) -> None:
        self.graph = graph
        self.run_task = run_task

    def run_all(self) -> None:
        """Execute every runnable task until no ready tasks remain.

        Loop: query graph.ready_tasks(); for each ready task move it through
        ready -> running; call run_task; mark it completed on success or
        failed otherwise. Failed tasks permanently block their downstream
        dependents, so the loop terminates once nothing else is ready. An
        exception from run_task is treated exactly like a False result.
        """
        while True:
            ready = self.graph.ready_tasks()
            if not ready:
                return

            for task in ready:
                self.graph.transition(task.task_id, TaskState.READY)
                self.graph.transition(task.task_id, TaskState.RUNNING)

                try:
                    success = self.run_task(task)
                except Exception:
                    success = False

                target = TaskState.COMPLETED if success else TaskState.FAILED
                self.graph.transition(task.task_id, target)