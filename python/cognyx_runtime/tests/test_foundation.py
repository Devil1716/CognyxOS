import json
import logging

import pytest

from cognyx_runtime.configuration import Environment, load_config
from cognyx_runtime.errors import ConfigurationError
from cognyx_runtime.logging import configure_logging
from cognyx_runtime.plugins import PLUGIN_API_VERSION, PluginManager


def test_configuration_loads_typed_defaults() -> None:
    assert load_config({}).environment is Environment.DEVELOPMENT


def test_configuration_rejects_invalid_environment() -> None:
    with pytest.raises(ConfigurationError):
        load_config({"COGNYX_ENV": "unsupported"})


def test_json_logging(capsys: pytest.CaptureFixture[str]) -> None:
    configure_logging()
    logging.getLogger("test").info("foundation initialized")
    assert json.loads(capsys.readouterr().err)["message"] == "foundation initialized"


def test_empty_plugin_manager_initializes() -> None:
    manager = PluginManager()
    assert manager.discover() == ()
    assert PLUGIN_API_VERSION == "1"
