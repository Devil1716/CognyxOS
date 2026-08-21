"""Planner -> executor -> capability-adapter -> provider/tool integration.

Proves the real components compose across the whole horizontal slice:

    TaskGraph -> TaskExecutor -> CapabilityAdapter
                -> real OllamaProvider (HTTP boundary faked)
                -> real FilesystemReadTool (real tmp_path boundary)

The graph is a small chain: a model.inference task with no dependencies, then
a filesystem.read task that requires the model task. Only the Ollama HTTP/network
boundary is faked (via the injected json_client seam); everything else is real.

Both completion and event privacy are asserted, and execution ordering is
recorded at the real capability boundaries rather than inferred from final
states alone.
"""

import json

from cognyx_runtime.events import EventBus

from core.executor.capability_adapter import CapabilityAdapter
from core.executor.task_executor import TaskExecutor
from core.models.ollama_provider import OllamaProvider
from core.planner.task_graph import Task, TaskGraph, TaskState
from core.tools.filesystem_read import FilesystemReadTool

# Deliberately recognizable markers so any accidental leakage would be obvious.
PROMPT = "PROMPT_LEAK_MARKER_what_does_the_notes_file_say"
FILE_CONTENT = "FILE_CONTENT_LEAK_MARKER_hello_from_real_disk"


class RecordingFilesystemReadTool(FilesystemReadTool):
    """Spy around the real FilesystemReadTool.

    Records when the filesystem capability boundary is hit, then delegates to
    the real implementation. It does not fake behavior - the real file read
    still happens - so the only faked boundary remains the Ollama HTTP client.
    """

    def __init__(self, allowed_root, order, **kwargs):
        super().__init__(allowed_root, **kwargs)
        self._order = order

    def execute(self, request):
        self._order.append("filesystem.read")
        return super().execute(request)


def test_planner_executor_capability_pipeline(tmp_path):
    events = []
    bus = EventBus()
    bus.subscribe(events.append)

    order: list[str] = []
    target = tmp_path / "notes.txt"

    # --- real Ollama provider; only the HTTP/network boundary is faked -----
    def fake_ollama(method, url, payload=None, timeout_seconds=5.0):
        assert payload is not None and payload["prompt"] == PROMPT
        order.append("model.inference")
        # The model's output materializes the file the next task reads. This
        # makes the ordering provable: the filesystem read cannot have run
        # before this point.
        target.write_text(FILE_CONTENT, encoding="utf-8")
        return {"model": "llama3.2", "response": FILE_CONTENT, "done": True}

    provider = OllamaProvider(
        model_id="llama3.2",
        event_bus=bus,
        publisher="model-provider-host",
        json_client=fake_ollama,
    )

    # --- real filesystem tool bound to a real temp directory ---------------
    tool = RecordingFilesystemReadTool(
        tmp_path,
        order,
        event_bus=bus,
        publisher="tool-host",
    )

    # --- a small two-task dependency graph ---------------------------------
    graph = TaskGraph()
    graph.add_task(
        Task(
            task_id="model-1",
            kind="model.inference",
            capabilities=("chat",),
            input={"request_id": "req-model-1", "prompt": PROMPT},
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

    # Before running, the file the filesystem step reads does not exist yet.
    assert not target.exists()

    TaskExecutor(graph, CapabilityAdapter(provider, tool)).run_all()

    # 1. Both tasks end completed.
    assert graph.task("model-1").state == TaskState.COMPLETED
    assert graph.task("read-1").state == TaskState.COMPLETED

    # 2 & 3. Ordering at the real capability boundaries: the model task ran
    # first, and the filesystem task could not have run before its model
    # dependency completed (it was neither ready nor had a file to read).
    assert order == ["model.inference", "filesystem.read"]

    # 4 & 5. Privacy: recognizable prompt/file markers never reach any event
    # payload, even though the real components published onto the shared bus.
    assert any(
        event.event_type == "org.cognyx.tool.executed" for event in events
    )
    serialized = json.dumps([event.payload for event in events])
    assert PROMPT not in serialized
    assert FILE_CONTENT not in serialized