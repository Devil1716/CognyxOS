"""BaseAgent optional memory-store summary recording after a run."""

from core.agents import AgentState, BaseAgent
from core.executor.capability_adapter import CapabilityAdapter
from core.memory.memory_store import MemoryStore
from core.models.ollama_provider import OllamaProvider
from core.planner.task_graph import Task, TaskGraph
from core.tools.filesystem_read import FilesystemReadTool


def _fixture(tmp_path):
    (tmp_path / "notes.txt").write_text("hello", encoding="utf-8")

    graph = TaskGraph()
    graph.add_task(
        Task(
            task_id="model-1",
            kind="model.inference",
            capabilities=("chat",),
            input={"request_id": "req-1", "prompt": "a prompt"},
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

    provider = OllamaProvider(
        model_id="llama3.2",
        json_client=lambda method, url, payload=None, timeout_seconds=5.0: {
            "model": "llama3.2",
            "response": "ok",
            "done": True,
        },
    )
    tool = FilesystemReadTool(tmp_path)
    adapter = CapabilityAdapter(provider, tool)

    return graph, adapter


def test_agent_with_memory_store_records_run_summary(tmp_path):
    graph, adapter = _fixture(tmp_path)
    store = MemoryStore()

    agent = BaseAgent(goal_id="goal-1", memory_store=store)
    agent.initialize("run-1")
    agent.run_task_graph(graph, adapter)

    assert agent.state == AgentState.COMPLETED

    summary = store.get(agent.run_id)
    assert summary["outcome"] == "completed"
    assert summary["completed_task_ids"] == ["model-1", "read-1"]


def test_agent_without_memory_store_still_runs(tmp_path):
    graph, adapter = _fixture(tmp_path)

    agent = BaseAgent(goal_id="goal-1")
    agent.initialize("run-1")
    agent.run_task_graph(graph, adapter)

    assert agent.state == AgentState.COMPLETED