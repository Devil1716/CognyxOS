import pytest

from core.planner.planner import Planner, PlannerError, Step
from core.planner.task_graph import TaskState

CAP_READ = "org.cognyx.filesystem.read"


def test_linear_chain_starts_with_first_step_ready():
    steps = [
        Step(
            label="one",
            kind="filesystem.read",
            capabilities=(CAP_READ,),
            input={"path": "a.txt"},
        ),
        Step(
            label="two",
            kind="filesystem.read",
            capabilities=(CAP_READ,),
            input={"path": "b.txt"},
            depends_on=("one",),
        ),
        Step(
            label="three",
            kind="filesystem.read",
            capabilities=(CAP_READ,),
            input={"path": "c.txt"},
            depends_on=("two",),
        ),
    ]

    graph = Planner().build_graph(steps, {CAP_READ})

    assert [task.task_id for task in graph.ready_tasks()] == ["one"]


def test_missing_capability_raises_before_building():
    steps = [
        Step(
            label="think",
            kind="model.inference",
            capabilities=("chat",),
            input={"request_id": "req-1", "prompt": "hi"},
        )
    ]

    with pytest.raises(PlannerError, match="chat"):
        Planner().build_graph(steps, {CAP_READ})


def test_unknown_dependency_label_raises():
    steps = [
        Step(
            label="a",
            kind="filesystem.read",
            capabilities=(CAP_READ,),
            input={"path": "a.txt"},
        ),
        Step(
            label="b",
            kind="filesystem.read",
            capabilities=(CAP_READ,),
            input={"path": "b.txt"},
            depends_on=("ghost",),
        ),
    ]

    with pytest.raises(PlannerError, match="ghost"):
        Planner().build_graph(steps, {CAP_READ})


def test_circular_dependency_raises():
    steps = [
        Step(
            label="a",
            kind="filesystem.read",
            capabilities=(CAP_READ,),
            depends_on=("b",),
        ),
        Step(
            label="b",
            kind="filesystem.read",
            capabilities=(CAP_READ,),
            depends_on=("a",),
        ),
    ]

    with pytest.raises(PlannerError, match="[Cc]ircular"):
        Planner().build_graph(steps, {CAP_READ})


def test_duplicate_label_raises():
    steps = [
        Step(label="dup", kind="filesystem.read", capabilities=(CAP_READ,)),
        Step(label="dup", kind="filesystem.read", capabilities=(CAP_READ,)),
    ]

    with pytest.raises(PlannerError, match="[Dd]uplicate"):
        Planner().build_graph(steps, {CAP_READ})


def test_diamond_dependency_resolution_matches_ready_tasks():
    steps = [
        Step(
            label="d",
            kind="filesystem.read",
            capabilities=(CAP_READ,),
            input={"path": "d.txt"},
        ),
        Step(
            label="b",
            kind="filesystem.read",
            capabilities=(CAP_READ,),
            input={"path": "b.txt"},
            depends_on=("d",),
        ),
        Step(
            label="c",
            kind="filesystem.read",
            capabilities=(CAP_READ,),
            input={"path": "c.txt"},
            depends_on=("d",),
        ),
        Step(
            label="a",
            kind="filesystem.read",
            capabilities=(CAP_READ,),
            input={"path": "a.txt"},
            depends_on=("b", "c"),
        ),
    ]

    graph = Planner().build_graph(steps, {CAP_READ})

    assert [task.task_id for task in graph.ready_tasks()] == ["d"]

    graph.transition("d", TaskState.COMPLETED)
    assert [task.task_id for task in graph.ready_tasks()] == ["b", "c"]

    graph.transition("b", TaskState.COMPLETED)
    graph.transition("c", TaskState.COMPLETED)
    assert [task.task_id for task in graph.ready_tasks()] == ["a"]


def test_task_ids_are_deterministic_and_dependencies_use_labels():
    steps = [
        Step(
            label="read-notes",
            kind="filesystem.read",
            capabilities=(CAP_READ,),
            input={"path": "notes.txt"},
        ),
        Step(
            label="write-summary",
            kind="filesystem.write",
            capabilities=("org.cognyx.filesystem.write",),
            input={"path": "summary.txt", "content": "x"},
            depends_on=("read-notes",),
        ),
    ]
    available = {CAP_READ, "org.cognyx.filesystem.write"}

    first = Planner().build_graph(steps, available)
    second = Planner().build_graph(steps, available)

    assert [task.task_id for task in first.tasks] == ["read-notes", "write-summary"]
    assert [task.task_id for task in second.tasks] == [
        task.task_id for task in first.tasks
    ]

    by_id = {task.task_id: task for task in first.tasks}
    assert by_id["read-notes"].dependencies == ()
    assert by_id["write-summary"].dependencies == ("read-notes",)