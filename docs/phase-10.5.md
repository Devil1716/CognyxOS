# Phase 10.5: Developer ecosystem + SDK

**Status:** IMPLEMENTED (plugin registry + sample echo plugin)  
**Last Updated:** 2026-08-14

## Overview

Plugins declare permissions, capabilities, resource quotas, filesystem
scopes, network access, and runtime requirements. They do **not** inherit
the user's full permission set. Execution is denied when disabled, over
quota, or out of scope.

CLI surface (implemented as `PluginRegistry` operations):

- cognyx plugin create|build|test|install|inspect|enable|disable|remove

Sample plugin `sample-echo`: one capability (`echo.say`), one agent role
permission (`agent.role.echo`), one workspace scope (`/Workspace/Artifacts`).

Existing `sdk/rust` remains the core SDK crate. This phase adds the plugin
lifecycle on top rather than replacing it.

## Security

Least privilege, audit log, checksum verify, upgrade + rollback.

## Next

Phase 11: production hardening + release.
