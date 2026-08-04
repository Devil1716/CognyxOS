"""
CognyxOS Virtual Bridge Manager

Purpose: Create and manage isolated virtual networks for VMs
with strict security boundaries.

Features:
- Per-runtime network isolation
- NAT with port forwarding
- Traffic shaping
- Network policy enforcement
"""

import subprocess
import json
from pathlib import Path
from dataclasses import dataclass
from typing import Optional, List, Dict
from enum import Enum


class NetworkType(Enum):
    ISOLATED = "isolated"      # No external access
    NAT = "nat"               # NATed internet access
    BRIDGED = "bridged"       # Direct physical network access
    MACVLAN = "macvlan"       # Unique MAC on physical network


@dataclass
class VirtualBridge:
    name: str
    network_type: NetworkType
    subnet: str
    gateway: str
    vlan_id: Optional[int]
    attached_vms: List[str]


@dataclass
class FirewallRule:
    direction: str  # "ingress" or "egress"
    action: str     # "allow" or "deny"
    protocol: str   # "tcp", "udp", "icmp", "any"
    src_cidr: Optional[str]
    dst_cidr: Optional[str]
    dst_port: Optional[int]


class BridgeManager:
    """Manages virtual bridges for CognyxOS VMs."""
    
    def __init__(self):
        self.bridges: Dict[str, VirtualBridge] = {}
        
    def create_bridge(self, name: str, network_type: NetworkType,
                      subnet: str, vlan_id: Optional[int] = None) -> VirtualBridge:
        """
        Create a virtual bridge for VM networking.
        
        Reasoning: Each runtime type gets isolated network segment
        to prevent cross-VM attacks and enable granular policies.
        """
        # Parse subnet to get gateway
        gateway = subnet.rsplit('.', 1)[0] + '.1'
        
        # Create bridge device
        subprocess.run(["ip", "link", "add", "name", name, "type", "bridge"], check=True)
        
        # Assign IP to bridge (gateway)
        subprocess.run([
            "ip", "addr", "add", f"{gateway}/24", "dev", name
        ], check=True)
        
        # Bring up bridge
        subprocess.run(["ip", "link", "set", name, "up"], check=True)
        
        # Configure VLAN if specified
        if vlan_id is not None:
            # Create VLAN interface on physical NIC
            phys_nic = self._get_default_nic()
            vlan_iface = f"{phys_nic}.{vlan_id}"
            subprocess.run([
                "ip", "link", "add", "link", phys_nic,
                "name", vlan_iface, "type", "vlan", "id", str(vlan_id)
            ], check=True)
            subprocess.run(["ip", "link", "set", vlan_iface, "master", name], check=True)
            subprocess.run(["ip", "link", "set", vlan_iface, "up"], check=True)
        
        # Enable IP forwarding for NAT networks
        if network_type == NetworkType.NAT:
            subprocess.run([
                "sysctl", "-w", "net.ipv4.ip_forward=1"
            ], check=True)
            
            # Setup NAT masquerading
            subprocess.run([
                "iptables", "-t", "nat", "-A", "POSTROUTING",
                "-s", subnet, "-o", self._get_default_nic(),
                "-j", "MASQUERADE"
            ], check=True)
        
        bridge = VirtualBridge(
            name=name,
            network_type=network_type,
            subnet=subnet,
            gateway=gateway,
            vlan_id=vlan_id,
            attached_vms=[]
        )
        self.bridges[name] = bridge
        
        return bridge
    
    def attach_vm_to_bridge(self, vm_id: str, bridge_name: str,
                            tap_name: Optional[str] = None) -> str:
        """
        Attach VM's TAP interface to bridge.
        
        Reasoning: TAP devices provide Ethernet-level connectivity
        between VMs and virtual bridges.
        """
        if bridge_name not in self.bridges:
            raise ValueError(f"Bridge {bridge_name} does not exist")
        
        bridge = self.bridges[bridge_name]
        
        # Create TAP device if not provided
        if tap_name is None:
            tap_name = f"tap-{vm_id}"
            subprocess.run([
                "ip", "tuntap", "add", "dev", tap_name, "mode", "tap", "user", "qemu"
            ], check=True)
        
        # Attach TAP to bridge
        subprocess.run(["ip", "link", "set", tap_name, "master", bridge_name], check=True)
        subprocess.run(["ip", "link", "set", tap_name, "up"], check=True)
        
        # Track attachment
        if vm_id not in bridge.attached_vms:
            bridge.attached_vms.append(vm_id)
        
        return tap_name
    
    def detach_vm_from_bridge(self, vm_id: str, bridge_name: str) -> None:
        """Detach VM from bridge."""
        if bridge_name not in self.bridges:
            return
        
        bridge = self.bridges[bridge_name]
        tap_name = f"tap-{vm_id}"
        
        # Remove TAP from bridge
        subprocess.run(["ip", "link", "set", tap_name, "nomaster"], check=True)
        subprocess.run(["ip", "link", "delete", tap_name], check=True)
        
        # Update tracking
        if vm_id in bridge.attached_vms:
            bridge.attached_vms.remove(vm_id)
    
    def add_firewall_rule(self, bridge_name: str, rule: FirewallRule) -> None:
        """
        Add firewall rule for bridge traffic.
        
        Reasoning: nftables provides high-performance filtering
        with atomic rule updates.
        """
        if bridge_name not in self.bridges:
            raise ValueError(f"Bridge {bridge_name} does not exist")
        
        bridge = self.bridges[bridge_name]
        
        # Build nftables rule
        chain = "input" if rule.direction == "ingress" else "output"
        table = "inet filter"
        
        # Construct rule components
        rule_parts = []
        
        if rule.protocol != "any":
            rule_parts.append(rule.protocol)
        
        if rule.src_cidr:
            rule_parts.extend(["ip", "saddr", rule.src_cidr])
        
        if rule.dst_cidr:
            rule_parts.extend(["ip", "daddr", rule.dst_cidr])
        
        if rule.dst_port:
            rule_parts.extend(["tcp", "dport", str(rule.dst_port)])
        
        rule_parts.append(rule.action)
        
        # Insert rule using nft
        cmd = ["nft", "add", "rule", table, chain] + rule_parts
        subprocess.run(cmd, check=True)
    
    def setup_port_forwarding(self, bridge_name: str, host_port: int,
                              vm_ip: str, vm_port: int) -> None:
        """
        Setup port forwarding from host to VM.
        
        Reasoning: Enables external access to specific VM services
        while maintaining network isolation.
        """
        if bridge_name not in self.bridges:
            raise ValueError(f"Bridge {bridge_name} does not exist")
        
        # DNAT rule for incoming traffic
        subprocess.run([
            "iptables", "-t", "nat", "-A", "PREROUTING",
            "-p", "tcp", "--dport", str(host_port),
            "-j", "DNAT", "--to-destination", f"{vm_ip}:{vm_port}"
        ], check=True)
        
        # Forward rule
        subprocess.run([
            "iptables", "-A", "FORWARD",
            "-p", "tcp", "-d", vm_ip, "--dport", str(vm_port),
            "-j", "ACCEPT"
        ], check=True)
    
    def delete_bridge(self, name: str) -> None:
        """Delete bridge and cleanup."""
        if name not in self.bridges:
            return
        
        bridge = self.bridges[name]
        
        # Detach all VMs first
        for vm_id in list(bridge.attached_vms):
            self.detach_vm_from_bridge(vm_id, name)
        
        # Bring down and delete bridge
        subprocess.run(["ip", "link", "set", name, "down"], check=True)
        subprocess.run(["ip", "link", "delete", name], check=True)
        
        del self.bridges[name]
    
    def _get_default_nic(self) -> str:
        """Get default physical NIC."""
        result = subprocess.run(
            ["ip", "route", "show", "default"],
            capture_output=True, text=True
        )
        # Parse output like "default via 192.168.1.1 dev eth0 ..."
        for part in result.stdout.split():
            if part.startswith("dev"):
                continue
            if len(part) > 2 and not part.isdigit():
                return part
        return "eth0"  # Fallback
    
    def list_bridges(self) -> List[VirtualBridge]:
        """List all configured bridges."""
        return list(self.bridges.values())
    
    def get_bridge_info(self, name: str) -> Optional[VirtualBridge]:
        """Get detailed bridge information."""
        return self.bridges.get(name)


# Example usage
if __name__ == "__main__":
    manager = BridgeManager()
    
    # Create isolated network for Windows VMs
    # windows_bridge = manager.create_bridge(
    #     name="cognyx-win-net",
    #     network_type=NetworkType.NAT,
    #     subnet="192.168.100.0/24",
    #     vlan_id=100
    # )
    
    # Create isolated network for macOS VMs
    # macos_bridge = manager.create_bridge(
    #     name="cognyx-mac-net",
    #     network_type=NetworkType.ISOLATED,
    #     subnet="192.168.101.0/24",
    #     vlan_id=101
    # )
    
    # Attach VM to bridge
    # tap = manager.attach_vm_to_bridge("windows-vm-001", "cognyx-win-net")
    
    # Add firewall rule
    # manager.add_firewall_rule("cognyx-win-net", FirewallRule(
    #     direction="egress",
    #     action="deny",
    #     protocol="tcp",
    #     src_cidr=None,
    #     dst_cidr="10.0.0.0/8",  # Block internal networks
    #     dst_port=None
    # ))
    
    print("Bridge manager initialized")
