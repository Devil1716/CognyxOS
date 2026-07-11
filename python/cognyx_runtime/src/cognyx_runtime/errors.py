"""Global, stable error taxonomy for all runtime modules."""


class CognyxError(Exception):
    """Base error for every CognyxOS runtime component."""


class ConfigurationError(CognyxError):
    """Raised when typed configuration validation fails."""


class DependencyResolutionError(CognyxError):
    """Raised when a service or plugin dependency cannot be resolved."""


class LifecycleError(CognyxError):
    """Raised when a lifecycle transition is not permitted."""


class AuthenticationError(CognyxError):
    """Raised when a local IPC capability token is invalid."""


class VersionNegotiationError(CognyxError):
    """Raised when no compatible service contract version exists."""


class PluginError(CognyxError):
    """Base error for plugin lifecycle failures."""


class PluginCompatibilityError(PluginError):
    """Raised when a plugin targets an unsupported API version."""
