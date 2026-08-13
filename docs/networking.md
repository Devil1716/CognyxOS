# CognyxOS Virtual Networking Architecture

> **Document ID:** ARCH-PHASE2-NETWORKING  
> **Version:** 1.0.0  

---

## 1. Network Modes & Inter-Runtime Firewall

```mermaid
graph LR
    Host[Linux Host Bridge / NAT] --> VM1[Windows VM - NAT]
    Host --> VM2[Linux Container - NAT]
    Host --> Isolated[Isolated VM - No Host Egress]
    
    VM1 -. Policy Check: can_communicate() .-> VM2
```

## 2. Policy Decision Protocol
`VirtualNetworkManager::can_communicate(source_id, target_id, port, protocol)` evaluates firewall rules before allowing network packets across runtimes.
