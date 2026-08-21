"""BaseAgent optional memory-store summary recording after a run."""

from core.agents import AgentState, BaseAgent
from core.executor.capability_adapter import CapabilityAdapter
from core.memory.memory_store import MemoryStore
from core.models.ollama_provider import OllamaProvider
from core.planner.task_graph import Task, TaskGraph, TaskState
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


class RecordingAdapter:
    """Delegates to a real adapter while recording which task IDs run."""

    def __init__(self, adapter):
        self._adapter = adapter
        self.called: list[str] = []

    def __call__(self, task):
        self.called.append(task.task_id)
        return self._adapter(task)


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


def test_resume_skip_previously_completed(tmp_path):
    store = MemoryStore()

    first = BaseAgent(goal_id="goal-1", agent_id="agent-1", memory_store=store)
    first_adapter = RecordingAdapter(_single_read_adapter(tmp_path))
    first.initialize("run-1")
    first.run_task_graph(
        _single_read_graph(tmp_path, "notes.txt", "read-file"),
        first_adapter,
    )
    assert first_adapter.called == ["read-file"]

    # Second run with resume enabled: read-file is pre-completed from history,
    # so the capability callable is never invoked for it.
    second = BaseAgent(goal_id="goal-1", agent_id="agent-1", memory_store=store)
    second_adapter = RecordingAdapter(_single_read_adapter(tmp_path))
    second.initialize("run-2")
    second.run_task_graph(
        _single_read_graph(tmp_path, "notes.txt", "read-file"),
        second_adapter,
        skip_completed_from_history=True,
    )

    assert second.state == AgentState.COMPLETED
    assert second_adapter.called == []  # never executed read-file again


def test_default_runs_all_tasks_again(tmp_path):
    store = MemoryStore()

    first = BaseAgent(goal_id="goal-1", agent_id="agent-1", memory_store=store)
    first_adapter = RecordingAdapter(_single_read_adapter(tmp_path))
    first.initialize("run-1")
    first.run_task_graph(
        _single_read_graph(tmp_path, "notes.txt", "read-file"),
        first_adapter,
    )
    assert first_adapter.called == ["read-file"]

    # Default (flag unset -> False) re-executes every task, even one completed
    # in the previous run: the feature is genuinely opt-in.
    second = BaseAgent(goal_id="goal-1", agent_id="agent-1", memory_store=store)
    second_adapter = RecordingAdapter(_single_read_adapter(tmp_path))
    second.initialize("run-2")
    second.run_task_graph(
        _single_read_graph(tmp_path, "notes.txt", "read-file"),
        second_adapter,
    )

    assert second.state == AgentState.COMPLETED
    assert second_adapter.called == ["read-file"]  # executed again


def test_resume_does_not_skip_new_task(tmp_path):
    store = MemoryStore()

    first = BaseAgent(goal_id="goal-1", agent_id="agent-1", memory_store=store)
    first_adapter = RecordingAdapter(_single_read_adapter(tmp_path))
    first.initialize("run-1")
    first.run_task_graph(
        _single_read_graph(tmp_path, "notes.txt", "old-task"),
        first_adapter,
    )
    assert first_adapter.called == ["old-task"]

    (tmp_path / "new.txt").write_text("hello", encoding="utf-8")
    second_graph = TaskGraph()
    second_graph.add_task(
        Task(
            task_id="old-task",
            kind="filesystem.read",
            capabilities=("org.cognyx.filesystem.read",),
            input={"path": "notes.txt"},
        )
    )
    second_graph.add_task(
        Task(
            task_id="new-task",
            kind="filesystem.read",
            capabilities=("org.cognyx.filesystem.read",),
            input={"path": "new.txt"},
            dependencies=("old-task",),
        )
    )

    second = BaseAgent(goal_id="goal-1", agent_id="agent-1", memory_store=store)
    second_adapter = RecordingAdapter(_single_read_adapter(tmp_path))
    second.initialize("run-2")
    second.run_task_graph(
        second_graph,
        second_adapter,
        skip_completed_from_history=True,
    )

    assert second.state == AgentState.COMPLETED
    assert second_adapter.called == ["new-task"]  # old-task skipped, new ran
    states = {task.task_id: task.state for task in second_graph.tasks}
    assert states["old-task"] == TaskState.COMPLETED
    assert states["new-task"] == TaskState.COMPLETED


def test_resume_without_memory_store_is_noop(tmp_path):
    agent = BaseAgent(goal_id="goal-1")  # no memory_store
    adapter = RecordingAdapter(_single_read_adapter(tmp_path))
    agent.initialize("run-1")
    agent.run_task_graph(
        _single_read_graph(tmp_path, "notes.txt", "read-file"),
        adapter,
        skip_completed_from_history=True,
    )

    assert agent.state == AgentState.COMPLETED
    assert adapter.called == ["read-file"]  # executed normally, nothing recalled


def _chain_graph(tmp_path, task_ids):
    graph = TaskGraph()
    previous = None
    for task_id in task_ids:
        dependencies = (previous,) if previous else ()
        file_name = f"file-{task_id}.txt"
        (tmp_path / file_name).write_text("hello", encoding="utf-8")
        graph.add_task(
            Task(
                task_id=task_id,
                kind="filesystem.read",
                capabilities=("org.cognyx.filesystem.read",),
                input={"path": file_name},
                dependencies=dependencies,
            )
        )
        previous = task_id
    return graph


def test_resume_history_survives_across_three_runs(tmp_path):
    store = MemoryStore()

    # Run 1: chain a -> b, nothing to resume.
    first = BaseAgent(goal_id="goal-1", agent_id="agent-1", memory_store=store)
    first_adapter = RecordingAdapter(_single_read_adapter(tmp_path))
    first.initialize("run-1")
    first.run_task_graph(_chain_graph(tmp_path, ["a", "b"]), first_adapter)
    assert first_adapter.called == ["a", "b"]

    # Run 2: chain a -> b -> c; a and b resumed, only c is new.
    second = BaseAgent(goal_id="goal-1", agent_id="agent-1", memory_store=store)
    second_adapter = RecordingAdapter(_single_read_adapter(tmp_path))
    second.initialize("run-2")
    second.run_task_graph(
        _chain_graph(tmp_path, ["a", "b", "c"]),
        second_adapter,
        skip_completed_from_history=True,
    )
    assert second_adapter.called == ["c"]

    # Run 3: chain a -> b -> c -> d; a, b, c must all be skipped, only d runs.
    graph3 = _chain_graph(tmp_path, ["a", "b", "c", "d"])
    third = BaseAgent(goal_id="goal-1", agent_id="agent-1", memory_store=store)
    third_adapter = RecordingAdapter(_single_read_adapter(tmp_path))
    third.initialize("run-3")
    third.run_task_graph(
        graph3,
        third_adapter,
        skip_completed_from_history=True,
    )

    assert third.state == AgentState.COMPLETED
    assert third_adapter.called == ["d"]  # a, b, c all skipped, not re-run
    states = {task.task_id: task.state for task in graph3.tasks}
    assert all(state == TaskState.COMPLETED for state in states.values())