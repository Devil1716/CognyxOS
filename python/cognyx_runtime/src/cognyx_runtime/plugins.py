"""Plugin discovery and lifecycle framework. No plugins are bundled in Phase 1."""

from dataclasses import dataclass
from importlib.metadata import entry_points
from typing import Protocol

from .errors import PluginCompatibilityError, PluginError

PLUGIN_API_VERSION = "1"


@dataclass(frozen=True, slots=True)
class PluginManifest:
    identifier: str
    version: str
    api_version: str
    dependencies: tuple[str, ...] = ()


class Plugin(Protocol):
    manifest: PluginManifest

    def start(self) -> None: ...
    def stop(self) -> None: ...


class PluginManager:
    def __init__(self) -> None:
        self._plugins: dict[str, Plugin] = {}

    def discover(self) -> tuple[str, ...]:
        return tuple(entry.name for entry in entry_points(group="cognyx.plugins"))

    def register(self, plugin: Plugin) -> None:
        manifest = plugin.manifest
        if manifest.api_version != PLUGIN_API_VERSION:
            raise PluginCompatibilityError(
                f"{manifest.identifier} targets API {manifest.api_version}"
            )
        if manifest.identifier in self._plugins:
            raise PluginError(f"Plugin already registered: {manifest.identifier}")
        missing = set(manifest.dependencies) - self._plugins.keys()
        if missing:
            raise PluginError(f"Missing dependencies for {manifest.identifier}: {sorted(missing)}")
        self._plugins[manifest.identifier] = plugin

    def load(self, identifier: str) -> None:
        self._plugins[identifier].start()

    def unload(self, identifier: str) -> None:
        self._plugins[identifier].stop()
