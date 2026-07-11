import json
from urllib.request import Request, urlopen

import pytest

from cognyx_runtime.console import render_startup
from cognyx_runtime.container import Container, Lifetime
from cognyx_runtime.errors import DependencyResolutionError, LifecycleError, VersionNegotiationError
from cognyx_runtime.events import Event, EventBus
from cognyx_runtime.lifecycle import LifecycleCoordinator, RuntimeState
from cognyx_runtime.registry import ServiceHealth, ServiceRecord, ServiceRegistry
from cognyx_runtime.runtime import Runtime
from cognyx_runtime.scheduler import ScheduledTask, Scheduler, TaskPriority


def test_container_supports_lifetimes_and_constructor_injection() -> None:
    class First:
        pass

    class Second:
        def __init__(self, first: First) -> None:
            self.first = first

    container = Container()
    container.register(First, First)
    container.register(Second, Second, Lifetime.TRANSIENT)
    assert container.resolve(Second).first is container.resolve(First)
    assert container.resolve(Second) is not container.resolve(Second)
    with pytest.raises(DependencyResolutionError):
        container.resolve(str)


def test_lifecycle_rejects_undocumented_transition() -> None:
    lifecycle = LifecycleCoordinator()
    with pytest.raises(LifecycleError):
        lifecycle.transition(RuntimeState.RUNNING, "skip", "boot")
    lifecycle.transition(RuntimeState.INITIALIZING, "boot", "boot")
    lifecycle.transition(RuntimeState.STARTING, "valid", "boot")
    assert (
        lifecycle.transition(RuntimeState.RUNNING, "ready", "boot").current is RuntimeState.RUNNING
    )


def test_registry_discovers_healthy_compatible_service() -> None:
    registry = ServiceRegistry()
    record = registry.register(ServiceRecord("events", "", ("1.2",), ("replay",), "local://events"))
    registry.report_health(record.instance_id, ServiceHealth.HEALTHY)
    assert registry.discover("events", "1.1", "replay").endpoint == "local://events"
    with pytest.raises(VersionNegotiationError):
        registry.discover("events", "2.0")


def test_event_bus_persists_filters_and_replays() -> None:
    bus = EventBus()
    received: list[Event] = []
    subscription = bus.subscribe(received.append, lambda event: event.event_type.endswith("booted"))
    published = bus.publish(Event("org.cognyx.system.booted", {"boot_id": "test"}, "runtime"))
    bus.publish(Event("org.cognyx.system.shutdown", {}, "runtime"))
    assert received == [published]
    assert len(bus.replay("org.cognyx.system.boot")) == 1
    bus.unsubscribe(subscription)
    with pytest.raises(ValueError):
        bus.publish(Event("invalid", {}, "runtime"))
    bus.close()


def test_scheduler_runs_priority_task_and_can_close_admission() -> None:
    scheduler = Scheduler(workers=1)
    scheduler.start()
    future = scheduler.schedule(ScheduledTask(lambda cancel: "done", priority=TaskPriority.HIGH))
    assert future.result() == "done"
    scheduler.pause()
    with pytest.raises(RuntimeError):
        scheduler.schedule(ScheduledTask(lambda cancel: "nope"))
    scheduler.shutdown()


def test_runtime_boots_serves_local_endpoints_and_stops() -> None:
    runtime = Runtime()
    runtime.start()
    assert runtime.lifecycle.state is RuntimeState.RUNNING
    assert "Runtime Status: READY" in render_startup(runtime)
    request = Request(
        runtime.api.endpoint + "/health", headers={"Authorization": f"Bearer {runtime.boot_id}"}
    )  # type: ignore[union-attr]
    with urlopen(request) as response:  # noqa: S310
        assert json.load(response) == {"live": True, "ready": True}
    runtime.stop()
    assert runtime.lifecycle.state is RuntimeState.STOPPED
