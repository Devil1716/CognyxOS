# Real golden path

The intended shell path is Shell `AgentKernelAdapter` -> `AgentKernelServer` -> IntentEngine -> TaskManager -> Planner -> ExecutionGraph -> GraphScheduler -> CapabilityGateway -> PermissionEngine -> RuntimeRegistry -> native provider.

Phase 13 adds structured planning and execution tracing (`validated execution plan`, per-node completion/failure) on that path. Phase 13.5 keeps that path and adds a fail-closed GUI test harness so live Notepad verification cannot target personal windows. Hardware GUI tests remain explicitly interactive/ignored. They must use the real Windows providers and a dedicated test-owned instance; RecordingKernel remains test-only and is not part of production shell execution.
