"""Cooperative priority scheduler for local background jobs."""

from collections.abc import Callable
from concurrent.futures import Future, ThreadPoolExecutor
from dataclasses import dataclass, field
from enum import IntEnum
from threading import Event, Lock
from time import monotonic
from uuid import uuid4


class TaskPriority(IntEnum):
    CRITICAL = 0
    HIGH = 1
    NORMAL = 2
    LOW = 3


@dataclass(slots=True)
class ScheduledTask:
    operation: Callable[[Event], object]
    priority: TaskPriority = TaskPriority.NORMAL
    timeout_seconds: float = 30
    retries: int = 0
    task_id: str = field(default_factory=lambda: str(uuid4()))
    cancellation: Event = field(default_factory=Event)


class Scheduler:
    def __init__(self, workers: int = 2) -> None:
        self._executor = ThreadPoolExecutor(
            max_workers=workers, thread_name_prefix="cognyx-scheduler"
        )
        self._tasks: dict[str, ScheduledTask] = {}
        self._futures: dict[str, Future[object]] = {}
        self._admission_open = False
        self._lock = Lock()

    def start(self) -> None:
        self._admission_open = True

    def pause(self) -> None:
        self._admission_open = False

    def schedule(self, task: ScheduledTask) -> Future[object]:
        if not self._admission_open:
            raise RuntimeError("Scheduler admission is closed")
        with self._lock:
            self._tasks[task.task_id] = task
            future = self._executor.submit(self._run, task)
            self._futures[task.task_id] = future
            return future

    def cancel(self, task_id: str) -> bool:
        task = self._tasks[task_id]
        task.cancellation.set()
        return self._futures[task_id].cancel()

    def shutdown(self) -> None:
        self.pause()
        for task in self._tasks.values():
            task.cancellation.set()
        self._executor.shutdown(wait=True, cancel_futures=True)

    def metrics(self) -> dict[str, object]:
        return {
            "queue_depth": sum(not future.done() for future in self._futures.values()),
            "admission_open": self._admission_open,
        }

    @staticmethod
    def _run(task: ScheduledTask) -> object:
        started = monotonic()
        for attempt in range(task.retries + 1):
            if task.cancellation.is_set():
                raise TimeoutError("Task cancelled")
            try:
                result = task.operation(task.cancellation)
                if monotonic() - started > task.timeout_seconds:
                    task.cancellation.set()
                    raise TimeoutError("Task deadline exceeded")
                return result
            except Exception:
                if attempt == task.retries:
                    raise
        raise RuntimeError("Unreachable scheduler state")
