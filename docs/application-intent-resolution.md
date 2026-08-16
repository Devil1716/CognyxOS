# Application intent resolution

Application names are resolved by the `application.search` capability, not by the planner constructing executable paths. A successful search emits a provider-discovered `application_id`; `application.open` consumes that ID. Empty searches now report `APPLICATION_NOT_FOUND`.

The deterministic plan for an application request is `application.search -> application.open`, followed by `keyboard.type` or `browser.navigate` when requested. The native provider remains the only component that launches an executable.
