import pytest
from cognyx_runtime.events import EventBus

from core.tools.filesystem_read import FilesystemReadTool, ReadToolError


def test_reads_text_file_inside_allowed_root(tmp_path):
    allowed_root = tmp_path
    target = allowed_root / "notes.txt"
    target.write_text("hello cognyx", encoding="utf-8")

    tool = FilesystemReadTool(allowed_root)
    result = tool.execute({"path": "notes.txt"})

    assert result["path"] == str(target.resolve())
    assert result["content"] == "hello cognyx"


def test_missing_file_is_resource_unavailable(tmp_path):
    tool = FilesystemReadTool(tmp_path)

    with pytest.raises(ReadToolError) as excinfo:
        tool.execute({"path": "does-not-exist.txt"})

    assert excinfo.value.failure_mode == "resource_unavailable"


def test_path_outside_allowed_root_is_rejected(tmp_path):
    tool = FilesystemReadTool(tmp_path / "sandbox")
    outside = tmp_path / "outside.txt"
    outside.write_text("secret", encoding="utf-8")

    with pytest.raises(ReadToolError) as excinfo:
        tool.execute({"path": str(outside)})

    assert excinfo.value.failure_mode == "validation_failed"


def test_invocation_publishes_audit_event(tmp_path):
    audited = []
    bus = EventBus()
    bus.subscribe(
        audited.append,
        predicate=lambda event: event.event_type == "org.cognyx.tool.executed",
    )

    target = tmp_path / "notes.txt"
    target.write_text("hello", encoding="utf-8")

    tool = FilesystemReadTool(tmp_path, event_bus=bus, publisher="test-publisher")
    tool.execute({"path": "notes.txt"})

    assert len(audited) == 1
    payload = audited[0].payload
    assert payload["tool_id"] == "org.cognyx.filesystem.read"
    assert payload["invocation_id"]
    assert payload["outcome"] == "completed"