"""Minimal constructor-injection container; modules declare rather than create dependencies."""

from collections.abc import Callable
from typing import Any

from .errors import DependencyResolutionError


class Container:
    def __init__(self) -> None:
        self._providers: dict[type[Any], Callable[[], Any]] = {}

    def register(self, contract: type[Any], provider: Callable[[], Any]) -> None:
        self._providers[contract] = provider

    def resolve(self, contract: type[Any]) -> Any:
        try:
            return self._providers[contract]()
        except KeyError as error:
            raise DependencyResolutionError(
                f"No provider registered for {contract.__name__}"
            ) from error
