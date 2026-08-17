"""BaseAgent.run_task_graph drives the planner/executor slice to completion."""

import json

from cognyx_runtime.events import EventBus

from core.agents import AgentState, BaseAgent
from core.executor.capability_adapter import CapabilityAdapter
from core.models.ollama_provider import OllamaProvider
from core.planner.task_graph import Task, TaskGraph
from core.tools.filesystem_read import FilesystemReadTool


class NeverCalledProvider:
    def inference(self, *args, **kwargs):
        raise AssertionError("model provider must not be invoked in this test")


def test_all_success_graph_completes(tmp_path):
    (tmp_path / "notes.txt").write_text("hello", encoding="utf-8")

    graph = TaskGraph()
    graph.add_task(
        Task(
            task_id="read-1",
            kind="filesystem.read",
            capabilities=("org.cognyx.filesystem.read",),
            input={"path": "notes.txt"},
        )
    )

    adapter = CapabilityAdapter(
        NeverCalledProvider(),
        FilesystemReadTool(tmp_path),
    )

    agent = BaseAgent(goal_id="goal-1")
    agent.initialize("run-1")
    agent.run_task_graph(graph, adapter)

    assert agent.state == AgentState.COMPLETED
    assert [record.current for record in agent.lifecycle.history] == [
        AgentState.INITIALIZING,
        AgentState.IDLE,
        AgentState.PLANNING,
        AgentState.REASONING,
        AgentState.EXECUTING,
        AgentState.OBSERVING,
        AgentState.IDLE,
        AgentState.COMPLETED,
    ]


def test_failed_task_fails_agent(tmp_path):
    graph = TaskGraph()
    graph.add_task(
        Task(
            task_id="missing-file",
            kind="filesystem.read",
            capabilities=("org.cognyx.filesystem.read",),
            input={"path": "does-not-exist.txt"},
        )
    )

    adapter = CapabilityAdapter(
        NeverCalledProvider(),
        FilesystemReadTool(tmp_path),
    )

    agent = BaseAgent(goal_id="goal-1")
    agent.initialize("run-1")
    agent.run_task_graph(graph, adapter)

    assert agent.state == AgentState.FAILED
    assert "missing-file" in agent.lifecycle.history[-1].reason


def test_event_payload_privacy(tmp_path):
    prompt = "PROMPT_LEAK_MARKER_secret_prompt"
    content = "FILE_CONTENT_LEAK_MARKER_secret_disk"
    (tmp_path / "notes.txt").write_text(content, encoding="utf-8")

    events = []
    bus = EventBus()
    bus.subscribe(events.append)

    provider = OllamaProvider(
        model_id="llama3.2",
        event_bus=bus,
        publisher="model-host",
        json_client=lambda method, url, payload=None, timeout_seconds=5.0: {
            "model": "llama3.2",
            "response": "ok",
            "done": True,
        },
    )
    tool = FilesystemReadTool(tmp_path, event_bus=bus, publisher="tool-host")

    graph = TaskGraph()
    graph.add_task(
        Task(
            task_id="model-1",
            kind="model.inference",
            capabilities=("chat",),
            input={"request_id": "req-1", "prompt": prompt},
        )
    )
    graph.add_task(
        Task(
            task_id="read-1",
            kind="filesystem.read",
            capabilities=("org.cognyx.filesystem.read",),
            input={"path": "notes.txt"},
            dependencies=("model-1",),
        )
    )

    agent = BaseAgent(goal_id="goal-1", event_bus=bus, publisher="agent-host")
    agent.initialize("run-1")
    agent.run_task_graph(graph, CapabilityAdapter(provider, tool))

    assert agent.state == AgentState.COMPLETED
    serialized = json.dumps([event.payload for event in events])
    assert prompt not in serialized
    assert content not in serialized