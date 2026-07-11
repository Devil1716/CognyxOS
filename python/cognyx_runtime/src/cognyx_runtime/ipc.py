"""Authenticated local development HTTP adapter.

Production service contracts remain protobuf-first.
"""

import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from threading import Thread
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .runtime import Runtime


class LocalApi:
    """Local-only diagnostics adapter. It binds loopback; no TCP exposure beyond development."""

    def __init__(self, runtime: "Runtime", token: str) -> None:
        self.runtime = runtime
        self.token = token
        self._server: ThreadingHTTPServer | None = None
        self._thread: Thread | None = None

    def start(self) -> None:
        api = self

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self) -> None:  # noqa: N802
                if self.headers.get("Authorization") != f"Bearer {api.token}":
                    self.send_response(401)
                    self.end_headers()
                    return
                routes = {
                    "/health": {
                        "live": api.runtime.health.liveness(),
                        "ready": api.runtime.health.ready(),
                    },
                    "/ready": {"ready": api.runtime.health.ready()},
                    "/metrics": api.runtime.diagnostics.metrics(),
                    "/runtime": api.runtime.runtime_info(),
                    "/services": [
                        record.__dict__
                        if hasattr(record, "__dict__")
                        else {name: getattr(record, name) for name in record.__slots__}
                        for record in api.runtime.registry.records()
                    ],
                    "/events": [event.event_type for event in api.runtime.events.events()],
                    "/diagnostics": api.runtime.diagnostics.snapshot(),
                }
                body = routes.get(self.path)
                if body is None:
                    self.send_response(404)
                    self.end_headers()
                    return
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps(body, default=str).encode())

            def log_message(self, format: str, *args: object) -> None:
                return

        self._server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self._thread = Thread(target=self._server.serve_forever, daemon=True)
        self._thread.start()

    @property
    def endpoint(self) -> str:
        if self._server is None:
            raise RuntimeError("IPC is not started")
        return f"http://127.0.0.1:{self._server.server_port}"

    def stop(self) -> None:
        if self._server:
            self._server.shutdown()
            self._server.server_close()
