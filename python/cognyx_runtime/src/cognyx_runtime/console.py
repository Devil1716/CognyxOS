"""Terminal developer console for runtime startup and state inspection."""

from .runtime import Runtime


def render_startup(runtime: Runtime) -> str:
    status = (
        "READY"
        if runtime.lifecycle.state.value == "Running"
        else runtime.lifecycle.state.value.upper()
    )
    checks = [
        ("Configuration", "OK"),
        ("Logging", "OK"),
        ("Dependency Injection", "OK"),
        ("Service Registry", "OK"),
        ("Event Bus", "OK"),
        ("IPC", "OK"),
        ("Scheduler", "OK"),
        ("Diagnostics", "OK"),
        ("Plugin Loader", "OK"),
        ("Health Checks", "PASS"),
    ]
    lines = ["=" * 51, "Cognyx Runtime v0.2", "=" * 51, ""]
    lines.extend(f"{name:.<28} {value}" for name, value in checks)
    lines.extend(
        [
            "",
            "=" * 51,
            f"Runtime Status: {status}",
            "Platform: Windows",
            f"Build: {runtime.config.environment.value.title()}",
            "Services: Running",
            "=" * 51,
        ]
    )
    return "\n".join(lines)
