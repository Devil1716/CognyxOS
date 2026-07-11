"""Durable SQLite event log with at-least-once local dispatch and replay."""

import json
import sqlite3
from collections.abc import Callable
from dataclasses import dataclass, replace
from datetime import UTC, datetime
from enum import StrEnum
from pathlib import Path
from uuid import uuid4


class EventPriority(StrEnum):
    CRITICAL = "critical"
    HIGH = "high"
    NORMAL = "normal"
    LOW = "low"


@dataclass(frozen=True, slots=True)
class Event:
    event_type: str
    payload: dict[str, object]
    publisher: str
    priority: EventPriority = EventPriority.NORMAL
    aggregate_id: str | None = None
    event_id: str = ""
    timestamp: str = ""
    replay: bool = False


class EventBus:
    def __init__(self, database_path: str = ":memory:") -> None:
        Path(database_path).parent.mkdir(
            parents=True, exist_ok=True
        ) if database_path != ":memory:" else None
        self._connection = sqlite3.connect(database_path, check_same_thread=False)
        self._connection.execute(
            "CREATE TABLE IF NOT EXISTS events ("
            "event_id TEXT PRIMARY KEY, event_type TEXT, payload TEXT, publisher TEXT, "
            "priority TEXT, aggregate_id TEXT, timestamp TEXT)"
        )
        self._connection.commit()
        self._subscriptions: dict[
            str, tuple[Callable[[Event], None], Callable[[Event], bool] | None]
        ] = {}
        self.published_total = 0

    def subscribe(
        self, handler: Callable[[Event], None], predicate: Callable[[Event], bool] | None = None
    ) -> str:
        identifier = str(uuid4())
        self._subscriptions[identifier] = (handler, predicate)
        return identifier

    def unsubscribe(self, identifier: str) -> None:
        self._subscriptions.pop(identifier, None)

    def publish(self, event: Event) -> Event:
        if not event.event_type.startswith("org.cognyx.") or not isinstance(event.payload, dict):
            raise ValueError("Event validation failed")
        persisted = replace(
            event,
            event_id=event.event_id or str(uuid4()),
            timestamp=event.timestamp or datetime.now(UTC).isoformat(),
        )
        self._connection.execute(
            "INSERT INTO events VALUES (?, ?, ?, ?, ?, ?, ?)",
            (
                persisted.event_id,
                persisted.event_type,
                json.dumps(persisted.payload),
                persisted.publisher,
                persisted.priority,
                persisted.aggregate_id,
                persisted.timestamp,
            ),
        )
        self._connection.commit()
        self.published_total += 1
        self._dispatch(persisted)
        return persisted

    def replay(self, event_type_prefix: str = "") -> tuple[Event, ...]:
        rows = self._connection.execute(
            "SELECT event_id,event_type,payload,publisher,priority,aggregate_id,timestamp "
            "FROM events WHERE event_type LIKE ? ORDER BY timestamp",
            (f"{event_type_prefix}%",),
        ).fetchall()
        events = tuple(
            Event(
                row[1],
                json.loads(row[2]),
                row[3],
                EventPriority(row[4]),
                row[5],
                row[0],
                row[6],
                True,
            )
            for row in rows
        )
        for event in events:
            self._dispatch(event)
        return events

    def events(self) -> tuple[Event, ...]:
        return self.replay()

    def close(self) -> None:
        """Release the local durable event-store handle after safe shutdown."""
        self._connection.close()

    def _dispatch(self, event: Event) -> None:
        for handler, predicate in self._subscriptions.values():
            if predicate is None or predicate(event):
                handler(event)
