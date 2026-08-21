import json

import pytest
from cognyx_runtime.events import EventBus

from core.tools.filesystem_write import FilesystemWriteTool, WriteToolError


def test_writes_text_inside_allowed_root(tmp_path):
    tool = FilesystemWriteTool(tmp_path)

    result = tool.execute({"path": "notes.txt", "content": "hello cognyx"})

    assert result["path"].endswith("notes.txt")
    assert result["bytes_written"] == len(b"hello cognyx")
    # verify the actual on-disk content, not just the return value
    assert (tmp_path / "notes.txt").read_text(encoding="utf-8") == "hello cognyx"


def test_path_outside_allowed_root_rejected_before_touching_disk(tmp_path):
    tool = FilesystemWriteTool(tmp_path / "sandbox")
    outside = tmp_path / "outside.txt"

    with pytest.raises(WriteToolError) as excinfo:
        tool.execute({"path": str(outside), "content": "secret"})

    assert excinfo.value.failure_mode == "validation_failed"
    assert not outside.exists()  # nothing written or created


def test_missing_parent_directory_fails_without_creating_dirs(tmp_path):
    tool = FilesystemWriteTool(tmp_path)

    with pytest.raises(WriteToolError) as excinfo:
        tool.execute({"path": "subdir/notes.txt", "content": "hello"})

    assert excinfo.value.failure_mode == "internal"
    assert not (tmp_path / "subdir").exists()


def test_audit_event_excludes_content(tmp_path):
    content = "SECRET_WRITE_LEAK_MARKER"
    events = []
    bus = EventBus()
    bus.subscribe(events.append)

    tool = FilesystemWriteTool(tmp_path, event_bus=bus, publisher="test-publisher")
    tool.execute({"path": "notes.txt", "content": content})

    audited = [
        event for event in events if event.event_type == "org.cognyx.tool.executed"
    ]
    assert len(audited) == 1
    assert audited[0].payload["tool_id"] == "org.cognyx.filesystem.write"
    assert audited[0].payload["invocation_id"]
    assert audited[0].payload["outcome"] == "completed"

    serialized = json.dumps([event.payload for event in events])
    assert content not in serialized


def test_descriptor_declares_write_semantics():
    tool = FilesystemWriteTool("unused-root")

    descriptor = tool.descriptor

    assert descriptor["capability_id"] == "org.cognyx.filesystem.write"
    assert descriptor["risk_level"] == "confirmation"
    assert descriptor["failure_modes"] == ["validation_failed", "internal"]
    assert descriptor["idempotency"] == "not_supported"
    assert descriptor["rollback"] == "unsupported"