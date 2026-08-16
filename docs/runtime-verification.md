# Runtime verification

| Runtime | Registered on this host | Class |
|---|---|---|
| Native Windows (`windows-host`, `WindowsRuntime::host()`) | yes | REAL + INTEGRATION VERIFIED (identity + PATH apps/processes) |
| LinuxRuntime as host | no (must not be the Windows host label) | N/A on this OS |
| Windows VM (Hyper-V) | probed; not claimed healthy unless State=Enabled | NOT VERIFIED / PERMISSION_DENIED / UNAVAILABLE as doctor reports |
| Docker / container | probed via `docker info` | UNAVAILABLE if daemon down; NOT_INSTALLED if binary missing |
| macOS VM | not present | RUNTIME_UNAVAILABLE if requested |
| Remote worker | local registry only | WAN NOT VERIFIED |

`LinuxRuntime.execute_command` and `WindowsAppAutomation::execute_powershell` remain formatted-string adapters from earlier phases. CapabilityGateway does **not** treat those as real execution (VAL-001).
