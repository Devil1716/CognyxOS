"""Local service registration, discovery, leases, health, and version negotiation."""

from dataclasses import dataclass, replace
from datetime import UTC, datetime, timedelta
from enum import StrEnum
from uuid import uuid4

from .errors import DependencyResolutionError, VersionNegotiationError


class ServiceHealth(StrEnum):
    REGISTERED = "registered"
    STARTING = "starting"
    HEALTHY = "healthy"
    UNHEALTHY = "unhealthy"
    DRAINING = "draining"
    STOPPED = "stopped"


@dataclass(frozen=True, slots=True)
class ServiceRecord:
    service_id: str
    instance_id: str
    contract_versions: tuple[str, ...]
    capabilities: tuple[str, ...]
    endpoint: str
    dependencies: tuple[str, ...] = ()
    health: ServiceHealth = ServiceHealth.REGISTERED
    lease_expires_at: datetime | None = None


def _version(value: str) -> tuple[int, int]:
    major, minor, *_ = value.lstrip("v").split(".")
    return int(major), int(minor)


class ServiceRegistry:
    def __init__(self) -> None:
        self._records: dict[str, ServiceRecord] = {}

    def register(self, record: ServiceRecord, lease_seconds: int = 30) -> ServiceRecord:
        if set(record.dependencies) - {item.service_id for item in self._records.values()}:
            raise DependencyResolutionError(f"Missing service dependencies: {record.dependencies}")
        registered = replace(
            record,
            instance_id=record.instance_id or str(uuid4()),
            lease_expires_at=datetime.now(UTC) + timedelta(seconds=lease_seconds),
        )
        self._records[registered.instance_id] = registered
        return registered

    def report_health(self, instance_id: str, health: ServiceHealth) -> ServiceRecord:
        record = self._records[instance_id]
        record = replace(record, health=health)
        self._records[instance_id] = record
        return record

    def renew(self, instance_id: str, lease_seconds: int = 30) -> ServiceRecord:
        record = replace(
            self._records[instance_id],
            lease_expires_at=datetime.now(UTC) + timedelta(seconds=lease_seconds),
        )
        self._records[instance_id] = record
        return record

    def discover(
        self, service_id: str, version: str, capability: str | None = None
    ) -> ServiceRecord:
        requested_major, requested_minor = _version(version)
        candidates = [
            record
            for record in self._records.values()
            if record.service_id == service_id
            and record.health is ServiceHealth.HEALTHY
            and (capability is None or capability in record.capabilities)
            and record.lease_expires_at
            and record.lease_expires_at > datetime.now(UTC)
        ]
        compatible = [
            record
            for record in candidates
            if any(
                major == requested_major and minor >= requested_minor
                for major, minor in map(_version, record.contract_versions)
            )
        ]
        if not compatible:
            raise VersionNegotiationError("VERSION_INCOMPATIBLE")
        return compatible[0]

    def records(self) -> tuple[ServiceRecord, ...]:
        return tuple(self._records.values())
