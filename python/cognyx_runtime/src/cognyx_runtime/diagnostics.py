"""Sanitized operational diagnostics and local metrics."""

from dataclasses import asdict
from time import monotonic
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .runtime import Runtime


class Diagnostics:
    def __init__(self, runtime: "Runtime") -> None:
        self._runtime = runtime
        self._started = monotonic()

    def snapshot(self) -> dict[str, object]:
        return {
            "boot_id": self._runtime.boot_id,
            "state": self._runtime.lifecycle.state,
            "uptime_seconds": round(monotonic() - self._started, 3),
            "health": [asdict(item) for item in self._runtime.health.inspect()],
            "services": [asdict(item) for item in self._runtime.registry.records()],
            "events_appended_total": self._runtime.events.published_total,
            "scheduler": self._runtime.scheduler.metrics(),
        }

    def metrics(self) -> dict[str, object]:
        snapshot = self.snapshot()
        return {
            "runtime.startup_duration_ms": self._runtime.startup_duration_ms,
            "events.appended_total": snapshot["events_appended_total"],
            "scheduler.queue_depth": snapshot["scheduler"]["queue_depth"],
            "health.score": 100 if self._runtime.health.ready() else 0,
        }
