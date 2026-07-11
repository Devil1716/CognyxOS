"""Constructor-injection container with approved runtime lifetimes."""

from collections.abc import Callable
from dataclasses import dataclass
from enum import StrEnum
from inspect import signature
from typing import Any

from .errors import DependencyResolutionError


class Lifetime(StrEnum):
    SINGLETON = "singleton"
    SCOPED = "scoped"
    TRANSIENT = "transient"


@dataclass(slots=True)
class Provider[T]:
    """Explicit lazy dependency provider; use only to break documented ordering cycles."""

    resolve: Callable[[], T]

    def get(self) -> T:
        return self.resolve()


@dataclass(slots=True)
class _Registration:
    factory: Callable[..., Any]
    lifetime: Lifetime
    instance: Any = None


class Container:
    def __init__(self) -> None:
        self._providers: dict[type[Any], _Registration] = {}
        self._scoped: dict[type[Any], Any] = {}
        self._resolving: list[type[Any]] = []

    def register(
        self,
        contract: type[Any],
        provider: Callable[..., Any] | type[Any],
        lifetime: Lifetime = Lifetime.SINGLETON,
    ) -> None:
        self._providers[contract] = _Registration(provider, lifetime)

    def scope(self) -> "Container":
        child = Container()
        child._providers = self._providers
        return child

    def resolve(self, contract: type[Any]) -> Any:
        if contract not in self._providers:
            raise DependencyResolutionError(f"No provider registered for {contract.__name__}")
        if contract in self._resolving:
            path = " -> ".join(item.__name__ for item in [*self._resolving, contract])
            raise DependencyResolutionError(f"Dependency cycle: {path}")
        registration = self._providers[contract]
        if registration.lifetime is Lifetime.SINGLETON and registration.instance is not None:
            return registration.instance
        if registration.lifetime is Lifetime.SCOPED and contract in self._scoped:
            return self._scoped[contract]
        self._resolving.append(contract)
        try:
            parameters = signature(registration.factory).parameters.values()
            dependencies = [
                self.resolve(param.annotation)
                for param in parameters
                if param.annotation is not param.empty
            ]
            value = registration.factory(*dependencies)
        finally:
            self._resolving.pop()
        if registration.lifetime is Lifetime.SINGLETON:
            registration.instance = value
        elif registration.lifetime is Lifetime.SCOPED:
            self._scoped[contract] = value
        return value
