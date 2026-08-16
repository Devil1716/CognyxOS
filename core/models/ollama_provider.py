"""Ollama model provider (first slice).

Implements the model provider contract from
docs/contracts/plugins-models.md for Ollama's local REST API at
http://localhost:11434.

This slice covers discovery, capability report, load, unload, non-streaming
inference, and health. Streaming and cancellation are intentionally deferred;
the capability report reports them as not supported.

Privacy: prompts and local model output are never placed in event payloads or
log lines. The provider publishes only the lifecycle events
``org.cognyx.model.loaded`` and ``org.cognyx.model.unloaded``. The health
probe and telemetry stay strictly on-device (loopback to the local Ollama
server; nothing is ever sent off-device).
"""

import json
import urllib.error
import urllib.request
from collections.abc import Callable
from typing import ClassVar
from uuid import uuid4

from cognyx_runtime.events import Event, EventBus


class OllamaProviderError(RuntimeError):
    """Ollama provider error carrying a failure mode.

    Mirrors ReadToolError from core/tools/filesystem_read.py so callers can
    map any provider failure to the standard failure-mode vocabulary.
    """

    def __init__(self, failure_mode: str, message: str) -> None:
        super().__init__(message)
        self.failure_mode = failure_mode
        self.message = message


def default_json_client(
    method: str,
    url: str,
    payload: dict[str, object] | None = None,
    timeout_seconds: float = 5.0,
) -> dict[str, object]:
    """Default stdlib HTTP client for the Ollama REST JSON API."""
    try:
        data = None if payload is None else json.dumps(payload).encode("utf-8")
        request = urllib.request.Request(
            url,
            data=data,
            method=method,
            headers={
                "Accept": "application/json",
                "Content-Type": "application/json",
            },
        )
        with urllib.request.urlopen(request, timeout=timeout_seconds) as response:
            text = response.read().decode("utf-8")
    except TimeoutError as exc:
        raise OllamaProviderError(
            "deadline_exceeded",
            "Request to Ollama timed out.",
        ) from exc
    except urllib.error.HTTPError as exc:
        if exc.code == 400:
            raise OllamaProviderError(
                "validation_failed",
                "Ollama rejected the request.",
            ) from exc
        if exc.code == 404:
            raise OllamaProviderError(
                "resource_unavailable",
                "Ollama could not find the resource.",
            ) from exc
        raise OllamaProviderError(
            "internal",
            f"Ollama returned HTTP error {exc.code}.",
        ) from exc
    except (urllib.error.URLError, OSError) as exc:
        raise OllamaProviderError(
            "resource_unavailable",
            "Ollama is not reachable.",
        ) from exc

    try:
        return json.loads(text)
    except json.JSONDecodeError as exc:
        raise OllamaProviderError(
            "internal",
            "Ollama returned malformed JSON.",
        ) from exc


class OllamaProvider:
    """Ollama model provider facade over the local REST API.

    Dependencies (json_client, event_bus, publisher) are constructor-injected,
    mirroring BaseAgent and FilesystemReadTool; the provider owns no secrets
    and instantiates no platform services.
    """

    provider_id = "ollama"
    version = "1.0"
    format = "gguf"
    # Ollama stores model blobs in its own managed store, so no local path is
    # managed by this provider; the descriptor reports None (not applicable).
    local_path: str | None = None
    supported_capabilities: tuple[str, ...] = ("chat",)
    context_limits: ClassVar[dict[str, object]] = {
        "context_window": 8192,
        "max_output_tokens": 512,
    }
    hardware_requirements: ClassVar[dict[str, object]] = {
        "accelerator": "any_local_device",
        "minimum_ram_gb": 8,
    }
    content_classification = "unclassified"

    def __init__(
        self,
        model_id: str = "llama3.2",
        *,
        base_url: str = "http://localhost:11434",
        timeout_seconds: float = 30.0,
        json_client: Callable[..., dict[str, object]] | None = None,
        event_bus: EventBus | None = None,
        publisher: str | None = None,
    ) -> None:
        if not model_id:
            raise ValueError("model_id is required.")
        if event_bus is not None and not publisher:
            raise ValueError(
                "publisher is required when event_bus is provided."
            )

        self.model_id = model_id
        self.base_url = base_url.rstrip("/")
        self.timeout_seconds = timeout_seconds
        self._json_client = json_client or default_json_client
        self.event_bus = event_bus
        self.publisher = publisher

        self._loaded = False
        self._instance_id: str | None = None
        self._load_result: dict[str, object] = {}

    @property
    def descriptor(self) -> dict[str, object]:
        """The model descriptor defined by the model provider contract."""
        return {
            "provider_id": self.provider_id,
            "model_id": self.model_id,
            "version": self.version,
            "format": self.format,
            "local_path": self.local_path,
            "supported_capabilities": list(self.supported_capabilities),
            "context_limits": self.context_limits,
            "hardware_requirements": self.hardware_requirements,
            "content_classification": self.content_classification,
        }

    def discover(self) -> list[dict[str, object]]:
        """Discover models available to the local Ollama server."""
        data = self._json_client(
            "GET",
            f"{self.base_url}/api/tags",
            timeout_seconds=min(self.timeout_seconds, 5.0),
        )
        models = data.get("models")
        if not isinstance(models, list):
            raise OllamaProviderError(
                "internal",
                "Ollama returned a malformed /api/tags response.",
            )
        return [
            {"model_id": item["name"]}
            for item in models
            if isinstance(item, dict) and "name" in item
        ]

    def capability_report(self) -> dict[str, object]:
        """Report the capabilities this provider supports right now."""
        return {
            "provider_id": self.provider_id,
            "model_id": self.model_id,
            "supported_capabilities": list(self.supported_capabilities),
            "streaming": False,
            "cancellation": False,
        }

    def _publish_event(
        self,
        event_type: str,
        payload: dict[str, object],
    ) -> None:
        """Publish a model lifecycle event when an event bus is configured.

        Mirrors BaseAgent._publish_event and FilesystemReadTool._publish_event:
        the bus is optional, and a publisher identity is required whenever a
        bus is present.
        """
        if self.event_bus is None:
            return

        if self.publisher is None:
            raise RuntimeError("Model event publisher is not configured.")

        self.event_bus.publish(
            Event(
                event_type=event_type,
                payload=payload,
                publisher=self.publisher,
                aggregate_id=self.provider_id,
            )
        )

    def load(self) -> dict[str, object]:
        """Load the model, emitting org.cognyx.model.loaded exactly once.

        Idempotent: every call after the first returns the original load
        result and does not emit a second event.
        """
        if self._loaded:
            return self._load_result

        if not self._server_available():
            raise OllamaProviderError(
                "resource_unavailable",
                "Ollama is not reachable.",
            )

        self._instance_id = str(uuid4())
        self._load_result = {
            "provider_id": self.provider_id,
            "model_id": self.model_id,
            "instance_id": self._instance_id,
            "loaded": True,
        }
        self._loaded = True
        self._publish_event(
            "org.cognyx.model.loaded",
            {
                "provider_id": self.provider_id,
                "model_id": self.model_id,
                "instance_id": self._instance_id,
            },
        )
        return self._load_result

    def unload(self, reason: str = "requested") -> dict[str, object]:
        """Unload the model, emitting ``org.cognyx.model.unloaded`` once.

        Idempotent: unloading an already-unloaded model is a no-op that does
        not emit a second event.
        """
        if not self._loaded:
            return {"unloaded": False}

        self._loaded = False
        self._instance_id = None
        self._publish_event(
            "org.cognyx.model.unloaded",
            {
                "provider_id": self.provider_id,
                "model_id": self.model_id,
                "reason": reason,
            },
        )
        return {
            "provider_id": self.provider_id,
            "model_id": self.model_id,
            "reason": reason,
            "unloaded": True,
        }

    def _server_available(self) -> bool:
        """Confirm the local Ollama server answers a lightweight probe."""
        try:
            self._json_client(
                "GET",
                f"{self.base_url}/api/tags",
                timeout_seconds=min(self.timeout_seconds, 5.0),
            )
            return True
        except OllamaProviderError:
            return False

    def inference(
        self,
        request_id: str,
        prompt: str,
        *,
        capability: str = "chat",
        parameters: dict[str, object] | None = None,
        correlation_id: str | None = None,
    ) -> dict[str, object]:
        """Run one non-streaming inference against the local Ollama server.

        CRITICAL: the prompt and the model output are never written to an
        event payload or a log line. No event is published for an inference
        call; only load/unload lifecycle events exist.
        """
        if not request_id:
            raise OllamaProviderError(
                "validation_failed",
                "request_id is required.",
            )
        if not isinstance(prompt, str) or not prompt:
            raise OllamaProviderError(
                "validation_failed",
                "prompt must be a non-empty string.",
            )
        if capability not in self.supported_capabilities:
            raise OllamaProviderError(
                "not_supported",
                f"Capability '{capability}' is not supported.",
            )

        payload: dict[str, object] = {
            "model": self.model_id,
            "prompt": prompt,
            "stream": False,
        }
        if parameters:
            payload["options"] = parameters

        data = self._json_client(
            "POST",
            f"{self.base_url}/api/generate",
            payload=payload,
            timeout_seconds=self.timeout_seconds,
        )

        response_text = data.get("response")
        if not isinstance(response_text, str):
            raise OllamaProviderError(
                "internal",
                "Ollama returned a malformed inference response.",
            )

        return {
            "provider_id": self.provider_id,
            "model_id": self.model_id,
            "request_id": request_id,
            "capability": capability,
            "correlation_id": correlation_id,
            "content": response_text,
            "done": bool(data.get("done", True)),
        }

    def health(self) -> str:
        """Report provider health: ``ready``, ``degraded``, or ``unavailable``.

        The probe targets the local Ollama server only; nothing is sent
        off-device, and the result is computed locally in memory.
        """
        try:
            data = self._json_client(
                "GET",
                f"{self.base_url}/api/tags",
                timeout_seconds=min(self.timeout_seconds, 5.0),
            )
        # A health probe must always answer (ready/degraded/unavailable) rather
        # than raise, regardless of what the injected JSON client raised.
        except Exception:  # noqa: BLE001
            return "unavailable"

        if not isinstance(data, dict):
            return "degraded"
        return "ready" if self._loaded else "degraded"