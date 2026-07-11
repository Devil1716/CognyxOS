"""Global, stable error taxonomy for all runtime modules."""


class CognyxError(Exception):
    """Base error for every CognyxOS runtime component."""


class ConfigurationError(CognyxError):
    """Raised when typed configuration validation fails."""


class DependencyResolutionError(CognyxError):
    """Raised when a service or plugin dependency cannot be resolved."""


class PluginError(CognyxError):
    """Base error for plugin lifecycle failures."""


class PluginCompatibilityError(PluginError):
    """Raised when a plugin targets an unsupported API version."""
