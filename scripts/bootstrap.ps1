[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
pnpm install
python -m pip install -e ./python/cognyx_runtime[test,docs]
Write-Host 'CognyxOS foundation bootstrapped.'
