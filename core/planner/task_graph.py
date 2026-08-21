"""Task graph (first slice): structure and dependency resolution.

Implements the task-graph portion of docs/contracts/lifecycles.md for the
"requires" edge only. It models immutable task nodes, their states, and the
dependency-resolution question - "what CAN run right now?".

Out of scope for this slice: the scheduler, parallel execution, retries,
rollback, the "blocks"/"compensates" edges, and durable transition logging.
A task node's state changes by producing a new frozen Task via
dataclasses.replace (the same pattern ServiceRegistry uses), because task
nodes are immutable by contract.
"""

from dataclasses import dataclass, field, replace
from enum import StrEnum
from threading import RLock


class TaskError(ValueError):
    """Raised when a task-graph operation is invalid."""


class TaskPriority(StrEnum):
    """Scheduling priority declared by the task graph contract."""

    CRITICAL = "critical"
    HIGH = "high"
    NORMAL = "normal"
    LOW = "low"


class TaskState(StrEnum):
    """Node states declared by the task graph contract."""

    PENDING = "pending"
    READY = "ready"
    RUNNING = "running"
    VERIFYING = "verifying"
    COMPLETED = "completed"
    RETRYING = "retrying"
    FAILED = "failed"
    CANCELLED = "cancelled"
    ROLLED_BACK = "rolled_back"


@dataclass(frozen=True, slots=True)
class Task:
    """An immutable task node.

    All fields are fixed at creation time. ``state`` changes are expressed as
    new Task instances produced by TaskGraph.transition, never by mutation.
    """

    task_id: str
    kind: str
    input: dict[str, object] = field(default_factory=dict)
    input_schema: str | None = None
    output_schema: str | None = None
    capabilities: tuple[str, ...] = ()
    priority: TaskPriority = TaskPriority.NORMAL
    deadline: str | None = None
    retry_policy: dict[str, object] | None = None
    cancellation_policy: dict[str, object] | None = None
    rollback_strategy: dict[str, object] | None = None
    dependencies: tuple[str, ...] = ()
    state: TaskState = TaskState.PENDING


class TaskGraph:
    """Holds task nodes and resolves their "requires" dependencies.

    A task is runnable only when it is still pending and every task it
    ``requires`` is completed. Scheduling those runnable tasks is left to a
    future scheduler; this class never transitions a task out of pending on
    its own.
    """

    def __init__(self) -> None:
        self._tasks: dict[str, Task] = {}
        self._lock = RLock()

    def add_task(self, task: Task) -> None:
        """Register a task node under its immutable task_id."""
        with self._lock:
            if task.task_id in self._tasks:
                raise TaskError(f"Task already exists: {task.task_id}")
            self._tasks[task.task_id] = task

    def task(self, task_id: str) -> Task:
        """Return the registered task node for ``task_id``."""
        with self._lock:
            if task_id not in self._tasks:
                raise TaskError(f"Unknown task: {task_id}")
            return self._tasks[task_id]

    @property
    def tasks(self) -> tuple[Task, ...]:
        """All registered tasks, in insertion order."""
        with self._lock:
            return tuple(self._tasks.values())

    def transition(self, task_id: str, target: TaskState) -> Task:
        """Produce and store a new Task with the requested state.

        The node is immutable, so this returns a fresh Task assembled with
        dataclasses.replace. Which transitions are legal is the concern of a
        future executor; this slice only records a requested state.
        """
        with self._lock:
            if task_id not in self._tasks:
                raise TaskError(f"Unknown task: {task_id}")
            updated = replace(self._tasks[task_id], state=target)
            self._tasks[task_id] = updated
            return updated

    def ready_tasks(self) -> tuple[Task, ...]:
        """Return pending tasks whose "requires" dependencies are satisfied.

        A task is runnable now if it is still pending and every task it
        depends on is completed. A dependency that is absent, pending,
        running, failed, cancelled, or rolled back leaves the task unrunnable.
        Results are returned in insertion order.
        """
        with self._lock:
            ready: list[Task] = []
            for task in self._tasks.values():
                if task.state != TaskState.PENDING:
                    continue
                if all(
                    self._tasks.get(dep) is not None
                    and self._tasks[dep].state == TaskState.COMPLETED
                    for dep in task.dependencies
                ):
                    ready.append(task)
            return tuple(ready)