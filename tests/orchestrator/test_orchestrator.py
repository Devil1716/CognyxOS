import pytest

from core.executor.capability_adapter import CapabilityAdapter
from core.orchestrator.orchestrator import Orchestrator, OrchestratorError
from core.planner.task_graph import Task, TaskGraph, TaskState
from core.tools.filesystem_read import FilesystemReadTool
from core.tools.filesystem_write import FilesystemWriteTool


class _StubProvider:
    def inference(self, *args, **kwargs):
        raise AssertionError("the orchestrator must never execute adapters")


class _StubTool:
    def execute(self, request):
        raise AssertionError("the orchestrator must never execute adapters")


def _make_adapter():
    return CapabilityAdapter(_StubProvider(), _StubTool())


def _graph(*tasks_capabilities):
    graph = TaskGraph()
    for index, capabilities in enumerate(tasks_capabilities):
        graph.add_task(
            Task(
                task_id=f"task-{index}",
                kind="test",
                capabilities=tuple(capabilities),
            )
        )
    return graph


def test_route_read_only_graph_returns_read_adapter():
    read_adapter = _make_adapter()
    orchestrator = Orchestrator({"org.cognyx.filesystem.read": read_adapter})

    graph = _graph(("org.cognyx.filesystem.read",))

    assert orchestrator.route(graph) is read_adapter


def test_route_write_only_graph_returns_write_adapter():
    write_adapter = _make_adapter()
    orchestrator = Orchestrator({"org.cognyx.filesystem.write": write_adapter})

    graph = _graph(("org.cognyx.filesystem.write",))

    assert orchestrator.route(graph) is write_adapter


def test_route_mixed_graph_with_different_adapters_raises():
    read_adapter = _make_adapter()
    write_adapter = _make_adapter()
    orchestrator = Orchestrator(
        {
            "org.cognyx.filesystem.read": read_adapter,
            "org.cognyx.filesystem.write": write_adapter,
        }
    )

    graph = _graph(
        ("org.cognyx.filesystem.read",),
        ("org.cognyx.filesystem.write",),
    )

    with pytest.raises(OrchestratorError, match="single adapter"):
        orchestrator.route(graph)


def test_route_empty_graph_raises():
    orchestrator = Orchestrator({"org.cognyx.filesystem.read": _make_adapter()})

    with pytest.raises(OrchestratorError, match="empty"):
        orchestrator.route(TaskGraph())


def test_route_missing_capability_registration_raises():
    orchestrator = Orchestrator({"org.cognyx.filesystem.read": _make_adapter()})

    graph = _graph(("org.cognyx.filesystem.write",))

    with pytest.raises(OrchestratorError, match="org.cognyx.filesystem.write"):
        orchestrator.route(graph)


def test_route_same_adapter_satisfies_multiple_capabilities():
    read_write_adapter = _make_adapter()
    orchestrator = Orchestrator(
        {
            "org.cognyx.filesystem.read": read_write_adapter,
            "org.cognyx.filesystem.write": read_write_adapter,
        }
    )

    graph = _graph(
        ("org.cognyx.filesystem.read",),
        ("org.cognyx.filesystem.write",),
    )

    assert orchestrator.route(graph) is read_write_adapter


class _FilesystemWriteAdapter:
    """Runs filesystem.write tasks against the real FilesystemWriteTool."""

    def __init__(self, tool):
        self._tool = tool

    def __call__(self, task):
        self._tool.execute(task.input)
        return True


def test_dispatch_read_only_task_to_read_adapter():
    read_adapter = _make_adapter()
    orchestrator = Orchestrator({"org.cognyx.filesystem.read": read_adapter})

    task = Task(
        task_id="read-1",
        kind="test",
        capabilities=("org.cognyx.filesystem.read",),
    )

    assert orchestrator.dispatch(task) is read_adapter


def test_dispatch_rejects_two_different_adapters():
    orchestrator = Orchestrator(
        {
            "org.cognyx.filesystem.read": _make_adapter(),
            "org.cognyx.filesystem.write": _make_adapter(),
        }
    )

    task = Task(
        task_id="mixed-1",
        kind="test",
        capabilities=("org.cognyx.filesystem.read", "org.cognyx.filesystem.write"),
    )

    with pytest.raises(OrchestratorError, match="single adapter"):
        orchestrator.dispatch(task)


def test_dispatch_rejects_zero_capability_task():
    orchestrator = Orchestrator({"org.cognyx.filesystem.read": _make_adapter()})

    task = Task(task_id="bare-1", kind="test")

    with pytest.raises(OrchestratorError, match="no capabilities"):
        orchestrator.dispatch(task)


def test_run_graph_executes_read_and_write_with_different_adapters(tmp_path):
    (tmp_path / "notes.txt").write_text("old content", encoding="utf-8")

    read_adapter = CapabilityAdapter(_StubProvider(), FilesystemReadTool(tmp_path))
    write_adapter = _FilesystemWriteAdapter(FilesystemWriteTool(tmp_path))
    orchestrator = Orchestrator(
        {
            "org.cognyx.filesystem.read": read_adapter,
            "org.cognyx.filesystem.write": write_adapter,
        }
    )

    graph = TaskGraph()
    graph.add_task(
        Task(
            task_id="read-1",
            kind="filesystem.read",
            capabilities=("org.cognyx.filesystem.read",),
            input={"path": "notes.txt"},
        )
    )
    graph.add_task(
        Task(
            task_id="write-1",
            kind="filesystem.write",
            capabilities=("org.cognyx.filesystem.write",),
            input={"path": "notes.txt", "content": "new content"},
        )
    )

    orchestrator.run_graph(graph)

    assert graph.task("read-1").state == TaskState.COMPLETED
    assert graph.task("write-1").state == TaskState.COMPLETED
    assert (tmp_path / "notes.txt").read_text(encoding="utf-8") == "new content"


def test_run_graph_unregistered_capability_leaves_task_failed(tmp_path):
    read_adapter = CapabilityAdapter(_StubProvider(), FilesystemReadTool(tmp_path))
    orchestrator = Orchestrator({"org.cognyx.filesystem.read": read_adapter})

    graph = TaskGraph()
    graph.add_task(
        Task(
            task_id="model-1",
            kind="model.inference",
            capabilities=("chat",),
            input={"request_id": "req-1", "prompt": "hi"},
        )
    )

    orchestrator.run_graph(graph)  # returns normally, does not raise

    assert graph.task("model-1").state == TaskState.FAILED