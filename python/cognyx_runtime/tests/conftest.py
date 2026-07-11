import pytest

from cognyx_runtime.configuration import Environment


@pytest.fixture
def test_environment() -> Environment:
    """Shared fixture for future runtime service tests."""
    return Environment.TEST
