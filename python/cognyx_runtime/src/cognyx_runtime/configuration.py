"""Validated environment-driven configuration."""

from collections.abc import Mapping
from dataclasses import dataclass
from enum import StrEnum
from os import environ

from .errors import ConfigurationError


class Environment(StrEnum):
    DEVELOPMENT = "development"
    TEST = "test"
    PRODUCTION = "production"


@dataclass(frozen=True, slots=True)
class AppConfig:
    environment: Environment
    log_level: str
    plugin_directory: str


def load_config(values: Mapping[str, str] | None = None) -> AppConfig:
    source = environ if values is None else values
    try:
        environment = Environment(source.get("COGNYX_ENV", Environment.DEVELOPMENT))
    except ValueError as error:
        raise ConfigurationError("COGNYX_ENV must be development, test, or production") from error
    log_level = source.get("COGNYX_LOG_LEVEL", "INFO").upper()
    if log_level not in {"DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"}:
        raise ConfigurationError("COGNYX_LOG_LEVEL is invalid")
    return AppConfig(environment, log_level, source.get("COGNYX_PLUGIN_DIR", "./plugins"))
