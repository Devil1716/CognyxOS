# Planner validation

Every plan is validated before graph execution. The validator enforces one capability per step, required capability inputs (`query`, `application_id`, `text`, `window_id`, `url`, `command`, or `destination` where applicable), and declared data-flow dependencies.

The Capability Gateway repeats required-input validation for direct capability callers. Permission checks remain subsequent and authoritative; plan validity does not grant a capability.
