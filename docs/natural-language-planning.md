# Natural-language planning

The deterministic intent provider recognizes `open` and `launch` requests before generic system or terminal patterns. It records `application`, `primary_action`, `actions`, optional `text` and `url`, expected outcome, and dependencies in structured intent parameters.

For example, `Open Notepad and type Hello CognyxOS` becomes application `Notepad`, actions `open,type`, and text `Hello CognyxOS`. Application intents never add `bash` or `terminal.execute` as a fallback.

`Open the browser` is intentionally rejected as `AMBIGUOUS_APPLICATION` when no configured default-browser policy is available. This avoids random selection.
