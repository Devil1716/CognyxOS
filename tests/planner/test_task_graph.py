import pytest

from core.planner.task_graph import Task, TaskError, TaskGraph, TaskState


def _task(task_id, dependencies=()):
    return Task(task_id=task_id, kind="test", dependencies=tuple(dependencies))


def test_task_with_no_dependencies_is_immediately_ready():
    graph = TaskGraph()
    graph.add_task(_task("a"))

    assert [task.task_id for task in graph.ready_tasks()] == ["a"]


def test_task_becomes_ready_only_after_dependency_completes():
    graph = TaskGraph()
    graph.add_task(_task("dep"))
    graph.add_task(_task("work", ["dep"]))

    # the pending dependency keeps the dependent unrunnable; only the
    # dependency itself is runnable right now.
    assert [task.task_id for task in graph.ready_tasks()] == ["dep"]

    graph.transition("dep", TaskState.COMPLETED)

    assert [task.task_id for task in graph.ready_tasks()] == ["work"]


def test_task_with_failed_dependency_never_becomes_ready():
    graph = TaskGraph()
    graph.add_task(_task("dep"))
    graph.add_task(_task("work", ["dep"]))

    graph.transition("dep", TaskState.FAILED)

    assert graph.ready_tasks() == ()


def test_diamond_dependencies_resolve_correctly():
    # a requires b and c; b and c each require d.
    graph = TaskGraph()
    graph.add_task(_task("d"))
    graph.add_task(_task("b", ["d"]))
    graph.add_task(_task("c", ["d"]))
    graph.add_task(_task("a", ["b", "c"]))

    assert [task.task_id for task in graph.ready_tasks()] == ["d"]

    graph.transition("d", TaskState.COMPLETED)
    assert [task.task_id for task in graph.ready_tasks()] == ["b", "c"]

    graph.transition("b", TaskState.COMPLETED)
    graph.transition("c", TaskState.COMPLETED)
    assert [task.task_id for task in graph.ready_tasks()] == ["a"]


def test_duplicate_task_id_is_rejected():
    graph = TaskGraph()
    graph.add_task(_task("a"))

    with pytest.raises(TaskError):
        graph.add_task(_task("a"))


def test_transition_of_unknown_task_raises():
    graph = TaskGraph()

    with pytest.raises(TaskError):
        graph.transition("missing", TaskState.COMPLETED)