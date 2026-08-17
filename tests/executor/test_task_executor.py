from core.executor.task_executor import TaskExecutor
from core.planner.task_graph import Task, TaskGraph, TaskState


def _task(task_id, dependencies=()):
    return Task(task_id=task_id, kind="test", dependencies=tuple(dependencies))


class RecordingGraph(TaskGraph):
    """TaskGraph that records every transition each task passes through."""

    def __init__(self) -> None:
        super().__init__()
        self.snapshots: dict[str, list[TaskState]] = {}

    def transition(self, task_id: str, target: TaskState) -> Task:
        self.snapshots.setdefault(task_id, []).append(target)
        return super().transition(task_id, target)


def test_linear_chain_executes_in_order():
    graph = TaskGraph()
    graph.add_task(_task("a"))
    graph.add_task(_task("b", ["a"]))
    graph.add_task(_task("c", ["b"]))

    order = []

    def run_task(task):
        order.append(task.task_id)
        return True

    TaskExecutor(graph, run_task).run_all()

    assert order == ["a", "b", "c"]
    assert [task.state for task in graph.tasks] == [
        TaskState.COMPLETED,
        TaskState.COMPLETED,
        TaskState.COMPLETED,
    ]


def test_diamond_graph_fully_resolves():
    graph = TaskGraph()
    graph.add_task(_task("d"))
    graph.add_task(_task("b", ["d"]))
    graph.add_task(_task("c", ["d"]))
    graph.add_task(_task("a", ["b", "c"]))

    order = []

    def run_task(task):
        order.append(task.task_id)
        return True

    TaskExecutor(graph, run_task).run_all()

    assert order == ["d", "b", "c", "a"]
    assert all(task.state == TaskState.COMPLETED for task in graph.tasks)


def test_failed_task_leaves_downstream_pending():
    graph = TaskGraph()
    graph.add_task(_task("a"))
    graph.add_task(_task("b", ["a"]))

    executed = []

    def run_task(task):
        executed.append(task.task_id)
        return False

    TaskExecutor(graph, run_task).run_all()

    assert executed == ["a"]
    assert graph.task("a").state == TaskState.FAILED
    assert graph.task("b").state == TaskState.PENDING


def test_run_task_exception_is_caught_and_fails_task():
    graph = TaskGraph()
    graph.add_task(_task("a"))
    graph.add_task(_task("b", ["a"]))

    executed = []

    def run_task(task):
        executed.append(task.task_id)
        if task.task_id == "a":
            raise RuntimeError("boom")
        return True

    TaskExecutor(graph, run_task).run_all()  # must not raise

    assert executed == ["a"]
    assert graph.task("a").state == TaskState.FAILED
    assert graph.task("b").state == TaskState.PENDING


def test_successful_task_follows_pending_ready_running_completed():
    graph = RecordingGraph()
    graph.add_task(_task("a"))

    def run_task(task):
        return True

    TaskExecutor(graph, run_task).run_all()

    assert graph.task("a").state == TaskState.COMPLETED
    assert graph.snapshots["a"] == [
        TaskState.READY,
        TaskState.RUNNING,
        TaskState.COMPLETED,
    ]


def test_failed_task_follows_pending_ready_running_failed():
    graph = RecordingGraph()
    graph.add_task(_task("a"))

    def run_task(task):
        return False

    TaskExecutor(graph, run_task).run_all()

    assert graph.task("a").state == TaskState.FAILED
    assert graph.snapshots["a"] == [
        TaskState.READY,
        TaskState.RUNNING,
        TaskState.FAILED,
    ]