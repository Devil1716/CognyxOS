import pytest
from cognyx_runtime.events import EventBus

from core.memory.memory_store import MemoryError, MemoryStore


def _bus():
    events = []
    bus = EventBus()
    bus.subscribe(events.append)
    return events, bus


def test_set_then_get():
    store = MemoryStore()
    store.set("greeting", "hello")
    assert store.get("greeting") == "hello"


def test_get_missing_raises_resource_unavailable():
    store = MemoryStore()

    with pytest.raises(MemoryError) as excinfo:
        store.get("nope")

    assert excinfo.value.failure_mode == "resource_unavailable"


def test_set_revisions_increment():
    events, bus = _bus()
    store = MemoryStore(event_bus=bus, publisher="mem-host")

    store.set("k", "v1")
    store.set("k", "v2")

    updated = [
        event for event in events if event.event_type == "org.cognyx.memory.updated"
    ]
    assert [event.payload["revision"] for event in updated] == [1, 2]
    assert [event.payload["operation"] for event in updated] == ["write", "write"]
    assert [event.payload["memory_id"] for event in updated] == ["k", "k"]


def test_delete_publishes_event_and_removes_value():
    events, bus = _bus()
    store = MemoryStore(event_bus=bus, publisher="mem-host")
    store.set("k", "v")
    events.clear()

    store.delete("k")

    updated = [
        event for event in events if event.event_type == "org.cognyx.memory.updated"
    ]
    assert len(updated) == 1
    assert updated[0].payload["operation"] == "delete"
    assert updated[0].payload["revision"] == 2  # next revision after set == 1

    with pytest.raises(MemoryError):
        store.get("k")


def test_delete_missing_is_noop():
    events, bus = _bus()
    store = MemoryStore(event_bus=bus, publisher="mem-host")

    store.delete("absent")  # no exception

    assert events == []


def test_revision_never_resets_after_delete():
    events, bus = _bus()
    store = MemoryStore(event_bus=bus, publisher="mem-host")

    store.set("k", "v1")  # revision 1
    store.delete("k")  # revision 2
    store.set("k", "v2")  # revision 3, not 1

    updated = [
        event for event in events if event.event_type == "org.cognyx.memory.updated"
    ]
    assert [event.payload["revision"] for event in updated] == [1, 2, 3]