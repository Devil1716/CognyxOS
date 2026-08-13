# CognyxOS Agent Kernel Daemon & Service Architecture

> **Document ID:** ARCH-PHASE3-AGENT-KERNEL  
> **Version:** 1.0.0  

---

## 1. Service Overview
The Agent Kernel Daemon (`cognyx-agent-kernel`) implements the `AgentKernelService` gRPC endpoint.
It integrates:
- `IntentEngine`
- `AgentTaskManager`
- `AgentPlanner`
- `GraphScheduler`
- `CapabilityGateway`
- `AgentMemoryEngine`

## 2. API Contract
- `ParseIntent`
- `SubmitAgentTask`
- `GetAgentTaskStatus`
- `GenerateExecutionGraph`
- `ExecuteGraph`
- `QueryAgentMemory`
- `StoreAgentMemory`
