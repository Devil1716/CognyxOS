import pytest
from cognyx_runtime.events import EventBus

from core.models.ollama_provider import OllamaProvider, OllamaProviderError


def _fake_client(response):
    def fake_client(method, url, payload=None, timeout_seconds=5.0):
        return response

    return fake_client


def test_load_publishes_loaded_event_once():
    events = []
    bus = EventBus()
    bus.subscribe(events.append)

    provider = OllamaProvider(
        model_id="llama3.2",
        event_bus=bus,
        publisher="test-provider",
        json_client=_fake_client({"models": [{"name": "llama3.2:latest"}]}),
    )

    result = provider.load()

    assert result["loaded"] is True
    loaded = [
        event
        for event in events
        if event.event_type == "org.cognyx.model.loaded"
    ]
    assert len(loaded) == 1
    assert loaded[0].publisher == "test-provider"
    assert loaded[0].payload == {
        "provider_id": "ollama",
        "model_id": "llama3.2",
        "instance_id": result["instance_id"],
    }


def test_load_is_idempotent():
    events = []
    bus = EventBus()
    bus.subscribe(events.append)

    provider = OllamaProvider(
        model_id="llama3.2",
        event_bus=bus,
        publisher="test-provider",
        json_client=_fake_client({"models": [{"name": "llama3.2:latest"}]}),
    )

    first = provider.load()
    second = provider.load()

    assert second == first
    assert second["instance_id"] == first["instance_id"]
    loaded = [
        event
        for event in events
        if event.event_type == "org.cognyx.model.loaded"
    ]
    assert len(loaded) == 1


def test_load_raises_when_ollama_is_unreachable():
    def fail_client(method, url, payload=None, timeout_seconds=5.0):
        raise OllamaProviderError("resource_unavailable", "connection refused")

    provider = OllamaProvider(model_id="llama3.2", json_client=fail_client)

    with pytest.raises(OllamaProviderError) as excinfo:
        provider.load()

    assert excinfo.value.failure_mode == "resource_unavailable"


def test_inference_returns_expected_output():
    captured = {}

    def fake_client(method, url, payload=None, timeout_seconds=5.0):
        captured["method"] = method
        captured["url"] = url
        captured["payload"] = payload
        return {"model": "llama3.2", "response": "Hello, world!", "done": True}

    provider = OllamaProvider(model_id="llama3.2", json_client=fake_client)

    result = provider.inference("req-1", "Say hello", correlation_id="corr-1")

    assert result["content"] == "Hello, world!"
    assert result["request_id"] == "req-1"
    assert result["provider_id"] == "ollama"
    assert captured["method"] == "POST"
    assert captured["url"].endswith("/api/generate")
    assert captured["payload"]["model"] == "llama3.2"
    assert captured["payload"]["prompt"] == "Say hello"
    assert captured["payload"]["stream"] is False


def test_inference_never_publishes_events():
    events = []
    bus = EventBus()
    bus.subscribe(events.append)

    provider = OllamaProvider(
        model_id="llama3.2",
        event_bus=bus,
        publisher="test-provider",
        json_client=_fake_client({"model": "llama3.2", "response": "Hello!"}),
    )
    provider.load()
    events.clear()

    result = provider.inference("req-1", "TOP SECRET PROMPT")

    assert result["content"] == "Hello!"
    assert events == []


def test_unload_publishes_unloaded_event_once():
    events = []
    bus = EventBus()
    bus.subscribe(events.append)

    provider = OllamaProvider(
        model_id="llama3.2",
        event_bus=bus,
        publisher="test-provider",
        json_client=_fake_client({"models": [{"name": "llama3.2:latest"}]}),
    )
    provider.load()
    events.clear()

    result = provider.unload(reason="shutdown")

    assert result["unloaded"] is True
    unloaded = [
        event
        for event in events
        if event.event_type == "org.cognyx.model.unloaded"
    ]
    assert len(unloaded) == 1
    assert unloaded[0].payload == {
        "provider_id": "ollama",
        "model_id": "llama3.2",
        "reason": "shutdown",
    }

    second = provider.unload(reason="shutdown")
    assert second["unloaded"] is False
    still_unloaded = [
        event
        for event in events
        if event.event_type == "org.cognyx.model.unloaded"
    ]
    assert len(still_unloaded) == 1


def test_health_reports_unavailable_when_ollama_is_unreachable():
    def fail_client(method, url, payload=None, timeout_seconds=5.0):
        raise OllamaProviderError("resource_unavailable", "connection refused")

    provider = OllamaProvider(model_id="llama3.2", json_client=fail_client)

    assert provider.health() == "unavailable"