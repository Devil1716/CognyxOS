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

def _single_read_graph(tmp_path, file_name, task_id):
    (tmp_path / file_name).write_text("hello", encoding="utf-8")
    graph = TaskGraph()
    graph.add_task(
        Task(
            task_id=task_id,
            kind="filesystem.read",
            capabilities=("org.cognyx.filesystem.read",),
            input={"path": file_name},
        )
    )
    return graph


class _NotUsedProvider:
    """Fails loudly if the model provider is ever invoked."""

    def inference(self, *args, **kwargs):
        raise AssertionError("model provider must not be invoked here")


def _single_read_adapter(tmp_path):
    return CapabilityAdapter(_NotUsedProvider(), FilesystemReadTool(tmp_path))


def test_recall_before_first_run_returns_none():
    store = MemoryStore()
    agent = BaseAgent(goal_id="goal-1", agent_id="agent-1", memory_store=store)

    assert agent.recall_previous_run() is None


def test_recall_after_first_run_returns_that_summary(tmp_path):
    store = MemoryStore()
    agent = BaseAgent(goal_id="goal-1", agent_id="agent-1", memory_store=store)
    agent.initialize("run-1")
    agent.run_task_graph(
        _single_read_graph(tmp_path, "notes.txt", "read-1"),
        _single_read_adapter(tmp_path),
    )

    assert agent.state == AgentState.COMPLETED

    summary = agent.recall_previous_run()
    assert summary is not None
    assert summary["outcome"] == "completed"
    assert summary["completed_task_ids"] == ["read-1"]


def test_recall_after_second_run_returns_second_not_first(tmp_path):
    store = MemoryStore()

    first = BaseAgent(goal_id="goal-1", agent_id="agent-1", memory_store=store)
    first.initialize("run-1")
    first.run_task_graph(
        _single_read_graph(tmp_path, "notes.txt", "read-1"),
        _single_read_adapter(tmp_path),
    )

    second = BaseAgent(goal_id="goal-2", agent_id="agent-1", memory_store=store)
    second.initialize("run-2")
    second.run_task_graph(
        _single_read_graph(tmp_path, "other.txt", "read-2"),
        _single_read_adapter(tmp_path),
    )

    # The agent_id key holds only the most recent summary (overwritten, not
    # appended), so recall from either agent instance sees the second run.
    assert second.recall_previous_run()["completed_task_ids"] == ["read-2"]
    assert first.recall_previous_run()["completed_task_ids"] == ["read-2"]


def test_recall_without_memory_store_returns_none(tmp_path):
    agent = BaseAgent(goal_id="goal-1", agent_id="agent-1")

    assert agent.recall_previous_run() is None

    agent.initialize("run-1")
    agent.run_task_graph(
        _single_read_graph(tmp_path, "notes.txt", "read-1"),
        _single_read_adapter(tmp_path),
    )

    assert agent.state == AgentState.COMPLETED
    assert agent.recall_previous_run() is None