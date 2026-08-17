import pytest

from core.executor.capability_adapter import CapabilityAdapter
from core.models.ollama_provider import OllamaProviderError
from core.planner.task_graph import Task
from core.tools.filesystem_read import ReadToolError


class FakeProvider:
    def __init__(self):
        self.calls = []
        self.error = None

    def inference(
        self,
        request_id,
        prompt,
        *,
        capability="chat",
        parameters=None,
        correlation_id=None,
    ):
        if self.error is not None:
            raise self.error
        self.calls.append(
            {
                "request_id": request_id,
                "prompt": prompt,
                "capability": capability,
                "correlation_id": correlation_id,
            }
        )
        return {"content": "ok"}


class FakeTool:
    def __init__(self):
        self.calls = []
        self.error = None

    def execute(self, request):
        if self.error is not None:
            raise self.error
        self.calls.append(request)
        return {"path": "/root/notes.txt", "content": "hello"}


def _task(kind, capabilities, input_data):
    return Task(task_id="t1", kind=kind, capabilities=capabilities, input=input_data)


def test_model_inference_task_succeeds():
    provider = FakeProvider()
    tool = FakeTool()
    adapter = CapabilityAdapter(provider, tool)

    task = _task("model.inference", ("chat",), {"request_id": "req-1", "prompt": "hello"})

    assert adapter(task) is True
    assert provider.calls == [
        {
            "request_id": "req-1",
            "prompt": "hello",
            "capability": "chat",
            "correlation_id": "t1",
        }
    ]
    assert tool.calls == []


def test_filesystem_read_task_succeeds():
    provider = FakeProvider()
    tool = FakeTool()
    adapter = CapabilityAdapter(provider, tool)

    task = _task(
        "filesystem.read",
        ("org.cognyx.filesystem.read",),
        {"path": "notes.txt"},
    )

    assert adapter(task) is True
    assert tool.calls == [{"path": "notes.txt"}]
    assert provider.calls == []


def test_filesystem_tool_error_returns_false():
    tool = FakeTool()
    tool.error = ReadToolError("resource_unavailable", "file missing")

    adapter = CapabilityAdapter(FakeProvider(), tool)
    task = _task(
        "filesystem.read",
        ("org.cognyx.filesystem.read",),
        {"path": "missing.txt"},
    )

    assert adapter(task) is False


def test_unrecognized_kind_returns_false():
    provider = FakeProvider()
    tool = FakeTool()
    adapter = CapabilityAdapter(provider, tool)

    task = _task("some.unknown", ("whatever",), {})

    assert adapter(task) is False
    assert provider.calls == []
    assert tool.calls == []


def test_provider_error_returns_false():
    provider = FakeProvider()
    provider.error = OllamaProviderError("resource_unavailable", "ollama down")

    adapter = CapabilityAdapter(provider, FakeTool())
    task = _task("model.inference", ("chat",), {"request_id": "req-1", "prompt": "hi"})

    assert adapter(task) is False


def test_kind_without_declared_capability_returns_false():
    provider = FakeProvider()
    tool = FakeTool()
    adapter = CapabilityAdapter(provider, tool)

    task = _task(
        "model.inference",
        ("org.cognyx.filesystem.read",),
        {"request_id": "req-1", "prompt": "hi"},
    )

    assert adapter(task) is False
    assert provider.calls == []
    assert tool.calls == []


def test_missing_required_input_field_propagates():
    adapter = CapabilityAdapter(FakeProvider(), FakeTool())
    task = _task("model.inference", ("chat",), {})

    with pytest.raises(KeyError):
        adapter(task)