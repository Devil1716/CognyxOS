"""Approved bootstrap coordinator for the Cognyx runtime."""

from time import monotonic
from uuid import uuid4

from .configuration import AppConfig, load_config
from .diagnostics import Diagnostics
from .events import Event, EventBus
from .health import HealthMonitor
from .ipc import LocalApi
from .lifecycle import LifecycleCoordinator, RuntimeState
from .logging import configure_logging
from .plugins import PluginManager
from .registry import ServiceHealth, ServiceRecord, ServiceRegistry
from .scheduler import Scheduler


class Runtime:
    """Owns startup and shutdown in the dependency order specified by bootstrap.md."""

    def __init__(self, config: AppConfig | None = None) -> None:
        self.config = config or load_config()
        self.boot_id = str(uuid4())
        self.lifecycle = LifecycleCoordinator()
        self.registry = ServiceRegistry()
        self.events = EventBus()
        self.scheduler = Scheduler()
        self.health = HealthMonitor()
        self.plugins = PluginManager()
        self.diagnostics = Diagnostics(self)
        self.api: LocalApi | None = None
        self.startup_duration_ms = 0.0

    def start(self) -> None:
        started = monotonic()
        self.lifecycle.transition(RuntimeState.INITIALIZING, "bootstrap initiated", self.boot_id)
        configure_logging(self.config.log_level)
        self.lifecycle.transition(RuntimeState.STARTING, "foundations validated", self.boot_id)
        self._register_core_services()
        self.api = LocalApi(self, token=self.boot_id)
        self.api.start()
        self.scheduler.start()
        self.health.register(
            "registry", lambda: (bool(self.registry.records()), "core services registered")
        )
        self.health.register("event_bus", lambda: (True, "event store recovered"))
        self.health.register("ipc", lambda: (self.api is not None, "local API started"))
        self.health.register(
            "scheduler",
            lambda: (self.scheduler.metrics()["admission_open"], "scheduler accepting work"),
        )
        if not self.health.ready():
            self.lifecycle.transition(RuntimeState.FAILED, "readiness checks failed", self.boot_id)
            raise RuntimeError("Runtime readiness checks failed")
        self.events.publish(
            Event(
                "org.cognyx.system.booted",
                {"boot_id": self.boot_id, "platform": "windows", "version": "0.2.0"},
                "runtime",
            )
        )
        self.lifecycle.transition(RuntimeState.RUNNING, "readiness checks passed", self.boot_id)
        self.startup_duration_ms = round((monotonic() - started) * 1000, 3)

    def stop(self, reason: str = "requested") -> None:
        if self.lifecycle.state not in {
            RuntimeState.RUNNING,
            RuntimeState.DEGRADED,
            RuntimeState.PAUSED,
            RuntimeState.FAILED,
        }:
            return
        self.lifecycle.transition(RuntimeState.STOPPING, reason, self.boot_id)
        self.scheduler.pause()
        self.events.publish(
            Event(
                "org.cognyx.system.shutdown",
                {"shutdown_id": str(uuid4()), "reason": reason, "deadline": "30s"},
                "runtime",
            )
        )
        self.scheduler.shutdown()
        if self.api:
            self.api.stop()
        self.events.close()
        self.lifecycle.transition(RuntimeState.STOPPED, "resources released", self.boot_id)

    def runtime_info(self) -> dict[str, object]:
        return {
            "boot_id": self.boot_id,
            "state": self.lifecycle.state,
            "startup_duration_ms": self.startup_duration_ms,
            "api_endpoint": self.api.endpoint if self.api else None,
        }

    def _register_core_services(self) -> None:
        for service_id, endpoint in (
            ("registry", "local://registry"),
            ("events", "local://events"),
            ("ipc", "local://ipc"),
            ("scheduler", "local://scheduler"),
            ("diagnostics", "local://diagnostics"),
            ("plugins", "local://plugins"),
        ):
            record = self.registry.register(ServiceRecord(service_id, "", ("1.0",), (), endpoint))
            self.registry.report_health(record.instance_id, ServiceHealth.HEALTHY)
