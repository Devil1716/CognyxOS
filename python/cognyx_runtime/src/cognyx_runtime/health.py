"""Liveness, readiness, and dependency health monitoring."""

from collections.abc import Callable
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class HealthCheck:
    name: str
    required: bool
    healthy: bool
    detail: str = ""


class HealthMonitor:
    def __init__(self) -> None:
        self._checks: dict[str, tuple[bool, Callable[[], tuple[bool, str]]]] = {}

    def register(
        self, name: str, callback: Callable[[], tuple[bool, str]], required: bool = True
    ) -> None:
        self._checks[name] = (required, callback)

    def inspect(self) -> tuple[HealthCheck, ...]:
        return tuple(
            HealthCheck(name, required, *callback())
            for name, (required, callback) in self._checks.items()
        )

    def ready(self) -> bool:
        return all(check.healthy or not check.required for check in self.inspect())

    def liveness(self) -> bool:
        return bool(self._checks)
