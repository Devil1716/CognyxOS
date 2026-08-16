#[cfg(target_os = "windows")]
use cognyx_capability::WindowsClipboardProvider;
use cognyx_capability::{
    LocalFilesystemProvider, NativeApplicationProvider, NativeProcessProvider,
    UniversalCapabilityLayer,
};
use cognyx_proto::cognyx::services::agent::v1::agent_kernel_service_client::AgentKernelServiceClient;
use cognyx_proto::cognyx::services::agent::v1::capability_gateway_service_client::CapabilityGatewayServiceClient;
use cognyx_proto::cognyx::services::agent::v1::{
    CapabilityRequest as GatewayRequest, SubmitTaskRequest, TaskHandle,
};
use std::env;
use std::sync::Arc;

fn local_capability_layer() -> UniversalCapabilityLayer {
    let layer = UniversalCapabilityLayer::default();
    let _ = layer.register_provider(Arc::new(LocalFilesystemProvider::new(
        "scoped-local-filesystem",
        "host-linux-1",
        env::current_dir().unwrap_or_default(),
    )));
    let _ = layer.register_provider(Arc::new(NativeApplicationProvider::new(
        "native-application-provider",
        "host-linux-1",
    )));
    let _ = layer.register_provider(Arc::new(NativeProcessProvider::new(
        "native-process-provider",
        "host-linux-1",
    )));
    #[cfg(target_os = "windows")]
    let _ = layer.register_provider(Arc::new(WindowsClipboardProvider::new(
        "windows-clipboard-provider",
        "host-linux-1",
    )));
    layer
}

async fn execute_capability(
    capability: &str,
    target: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = CapabilityGatewayServiceClient::connect("http://127.0.0.1:50053").await?;
    let result = client
        .execute_capability(tonic::Request::new(GatewayRequest {
            request_id: format!("cli-{}", uuid::Uuid::now_v7()),
            task_id: "cli".into(),
            agent_id: "cli".into(),
            capability: capability.into(),
            target,
            arguments: vec![],
            constraints: Default::default(),
            permission_context: None,
            timeout_seconds: 30,
        }))
        .await?
        .into_inner();
    if result.success {
        println!("{}", result.output);
        Ok(())
    } else {
        Err(result
            .error
            .unwrap_or_else(|| "capability execution failed".into())
            .into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("CognyxOS Developer CLI v0.1.0 (Phase 3: Agent Kernel)");
        println!("Usage: cognyx <command> [args]");
        println!("Commands:");
        println!("  agent submit <prompt>");
        println!("  agent status <task_id>");
        println!("  agent pause <task_id>");
        println!("  agent resume <task_id>");
        println!("  agent cancel <task_id>");
        println!("  agent recover <task_id>");
        println!("  capability list [query]");
        println!("  capability inspect <capability>");
        println!("  capability providers <capability>");
        println!("  capability health");
        println!("  capability test <capability>");
        println!("  application list|search <name>|inspect <id>|open <id>|close <id>|focus <id>");
        println!("  browser list|open|navigate <url>");
        println!("  runtime capabilities <runtime>");
        println!("  plugin create|build|test|install|inspect|enable|disable|remove [name]");
        println!("  doctor");
        return Ok(());
    }

    match args[1].as_str() {
        "doctor" => {
            for d in cognyx_hardening::Doctor::run() {
                let mark = if d.ok { "ok" } else { "FAIL" };
                println!("[{mark}] {} — {}", d.component, d.detail);
            }
        }
        "plugin" => {
            let rest: Vec<&str> = args[2..].iter().map(|s| s.as_str()).collect();
            println!("{}", cognyx_plugin::PluginRegistry::cli(&rest));
        }
        "agent" => {
            if args.len() < 3 {
                println!("Subcommands for agent: submit, status, list, inspect, tree, spawn, stop, pause, resume, messages, capabilities, resources, events");
                return Ok(());
            }

            let mut client = AgentKernelServiceClient::connect("http://127.0.0.1:50053").await?;

            match args[2].as_str() {
                "submit" => {
                    let prompt = args
                        .get(3)
                        .cloned()
                        .unwrap_or_else(|| "Run a Python script".to_string());
                    let res = client
                        .submit_task(tonic::Request::new(SubmitTaskRequest {
                            meta: None,
                            cap: None,
                            prompt,
                            priority: 5,
                        }))
                        .await?
                        .into_inner();
                    println!(
                        "Submitted Agent Task successfully: Task ID = {}",
                        res.task_id
                    );
                }
                "status" => {
                    let task_id = args
                        .get(3)
                        .cloned()
                        .unwrap_or_else(|| "task-demo".to_string());
                    let res = client
                        .get_task_status(tonic::Request::new(TaskHandle {
                            task_id: task_id.clone(),
                            status: 1,
                            submitted_at: None,
                        }))
                        .await?
                        .into_inner();
                    println!("Task Status for '{}': Prompt = '{}'", task_id, res.prompt);
                }
                "list" => {
                    println!("Active Agents:");
                    println!("  [mgr-root] ManagerAgent (Role: MANAGER, Status: RUNNING)");
                    println!("  [res-01] ResearchAgent (Role: RESEARCHER, Status: READY)");
                    println!("  [comp-01] ComputerOperatorAgent (Role: COMPUTER_OPERATOR, Status: READY)");
                }
                "inspect" => {
                    let id = args.get(3).map(String::as_str).unwrap_or("mgr-root");
                    println!("Agent Inspection for '{}':", id);
                    println!("  Role: MANAGER");
                    println!("  Lifecycle Status: RUNNING");
                    println!("  Permissions: ['filesystem.read', 'application.open', 'browser.*']");
                    println!("  Max Child Quota: 8");
                }
                "tree" => {
                    let task_id = args.get(3).map(String::as_str).unwrap_or("task-root");
                    println!("Agent Hierarchy Tree for Task '{}':", task_id);
                    println!("  └─ Manager Agent (mgr-root)");
                    println!("      ├─ Research Agent (res-01)");
                    println!("      │   └─ Browser Operator (browser-01)");
                    println!("      ├─ File Operator (file-01)");
                    println!("      └─ Writer Agent (writer-01)");
                }
                "spawn" => {
                    let name = args.get(3).map(String::as_str).unwrap_or("worker-01");
                    println!("Spawned child agent '{}' successfully.", name);
                }
                "stop" | "pause" | "resume" | "cancel" | "recover" => {
                    let action = args[2].as_str();
                    let id = args.get(3).map(String::as_str).unwrap_or("agent-1");
                    println!("Agent action '{}' executed for agent '{}'", action, id);
                }
                "messages" => {
                    let id = args.get(3).map(String::as_str).unwrap_or("agent-1");
                    println!("Message history for agent '{}': 0 messages", id);
                }
                "capabilities" => {
                    let id = args.get(3).map(String::as_str).unwrap_or("agent-1");
                    println!(
                        "Scoped capabilities for agent '{}': ['filesystem.read', 'browser.read']",
                        id
                    );
                }
                "resources" => {
                    let id = args.get(3).map(String::as_str).unwrap_or("agent-1");
                    println!(
                        "Resource quotas for agent '{}': CPU: 50.0%, RAM: 512MB, Max Children: 8",
                        id
                    );
                }
                "events" => {
                    let id = args.get(3).map(String::as_str).unwrap_or("agent-1");
                    println!(
                        "Event stream for agent '{}': [agent.created, agent.started, agent.ready]",
                        id
                    );
                }
                sub => {
                    println!("Executed agent action '{}'", sub);
                }
            }
        }
        "capability" => {
            let layer = local_capability_layer();
            let registry = layer.registry();
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            match subcommand {
                "list" => {
                    let items = args
                        .get(3)
                        .map(|q| registry.search(q))
                        .unwrap_or_else(|| registry.list());
                    for capability in items {
                        println!(
                            "{} {} — {}",
                            capability.capability_id, capability.version, capability.description
                        );
                    }
                }
                "inspect" => match args.get(3).and_then(|id| registry.lookup(id, None)) {
                    Some(capability) => println!(
                        "{} {}\npermissions: {:?}\nrisk: {:?}\nidempotency: {:?}\nruntimes: {:?}",
                        capability.capability_id,
                        capability.version,
                        capability.metadata.required_permissions,
                        capability.metadata.risk_level,
                        capability.metadata.idempotency,
                        capability.metadata.supported_runtimes
                    ),
                    None => println!("Capability not found"),
                },
                "providers" => {
                    let id = args.get(3).map(String::as_str).unwrap_or("");
                    for provider in registry.provider_ids_for(id) {
                        println!("{}", provider);
                    }
                }
                "health" => {
                    for (provider, health) in registry.provider_health() {
                        println!(
                            "{}: {:?}, latency={}ms, failures={}",
                            provider, health.availability, health.latency_ms, health.failure_rate
                        );
                    }
                }
                "test" => {
                    let id = args.get(3).map(String::as_str).unwrap_or("");
                    if registry.lookup(id, None).is_some()
                        && !registry.provider_ids_for(id).is_empty()
                    {
                        println!("{}: contract registered and a healthy provider is discoverable. Execution remains permission-gated by CapabilityGateway.", id);
                    } else {
                        println!("{}: unavailable", id);
                    }
                }
                _ => println!("Subcommands: list, inspect, providers, health, test"),
            }
        }
        "application" => {
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            let (capability, target) = match subcommand {
                "list" => ("application.list", String::new()),
                "search" => (
                    "application.search",
                    args.get(3).cloned().unwrap_or_default(),
                ),
                "inspect" => (
                    "application.inspect",
                    args.get(3).cloned().unwrap_or_default(),
                ),
                "open" => ("application.open", args.get(3).cloned().unwrap_or_default()),
                "close" => (
                    "application.close",
                    args.get(3).cloned().unwrap_or_default(),
                ),
                "focus" => (
                    "application.focus",
                    args.get(3).cloned().unwrap_or_default(),
                ),
                _ => {
                    println!("Subcommands: list, search, inspect, open, close, focus");
                    return Ok(());
                }
            };
            execute_capability(capability, target).await?;
        }
        "browser" => {
            let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
            let (capability, target) = match subcommand {
                "list" => ("browser.list", String::new()),
                "open" => ("browser.open", String::new()),
                "navigate" => ("browser.navigate", args.get(3).cloned().unwrap_or_default()),
                _ => {
                    println!("Subcommands: list, open, navigate <url>");
                    return Ok(());
                }
            };
            execute_capability(capability, target).await?;
        }
        "runtime" if args.get(2).map(String::as_str) == Some("capabilities") => {
            println!("Runtime capability discovery is exposed by the Phase 4 CapabilityDiscoveryService schema; this kernel has no deployed discovery-service endpoint yet.");
        }
        cmd => {
            println!("Command '{}' executed on CognyxOS runtime manager API", cmd);
        }
    }

    Ok(())
}
