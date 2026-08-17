import pytest

from core.executor.capability_adapter import CapabilityAdapter
from core.orchestrator.orchestrator import Orchestrator, OrchestratorError
from core.planner.task_graph import Task, TaskGraph


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