use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc_path = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc_path);

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    let proto_root = out_dir.join("proto");

    let common_dir = proto_root.join("cognyx/common/v1");
    let bus_dir = proto_root.join("cognyx/bus/v1");
    let core_dir = proto_root.join("cognyx/services/core/v1");
    let ai_dir = proto_root.join("cognyx/services/ai/v1");
    let security_dir = proto_root.join("cognyx/services/security/v1");

    let runtime_dir = proto_root.join("cognyx/services/runtime/v1");
    let capability_dir = proto_root.join("cognyx/services/capability/v1");

    fs::create_dir_all(&common_dir)?;
    fs::create_dir_all(&bus_dir)?;
    fs::create_dir_all(&core_dir)?;
    fs::create_dir_all(&ai_dir)?;
    fs::create_dir_all(&security_dir)?;
    fs::create_dir_all(&runtime_dir)?;
    fs::create_dir_all(&capability_dir)?;

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    // CARGO_MANIFEST_DIR is <root>/proto/gen/rust. Going up two levels gives <root>/proto
    let source_root = manifest_dir.join("../..");

    let agent_dir = proto_root.join("cognyx/services/agent/v1");
    fs::create_dir_all(&agent_dir)?;

    fs::copy(
        source_root.join("messages/common.proto"),
        common_dir.join("common.proto"),
    )?;
    fs::copy(
        source_root.join("messages/bus.proto"),
        bus_dir.join("bus.proto"),
    )?;
    fs::copy(
        source_root.join("services/core_services.proto"),
        core_dir.join("core_services.proto"),
    )?;
    fs::copy(
        source_root.join("services/ai_services.proto"),
        ai_dir.join("ai_services.proto"),
    )?;
    fs::copy(
        source_root.join("services/security_services.proto"),
        security_dir.join("security_services.proto"),
    )?;
    fs::copy(
        source_root.join("services/runtime_services.proto"),
        runtime_dir.join("runtime_services.proto"),
    )?;
    fs::copy(
        source_root.join("services/capability_services.proto"),
        capability_dir.join("capability_services.proto"),
    )?;
    fs::copy(
        source_root.join("services/agent_services.proto"),
        agent_dir.join("agent_services.proto"),
    )?;
    fs::copy(
        source_root.join("services/agent_runtime_services.proto"),
        agent_dir.join("agent_runtime_services.proto"),
    )?;

    let proto_files = [
        common_dir.join("common.proto"),
        bus_dir.join("bus.proto"),
        core_dir.join("core_services.proto"),
        ai_dir.join("ai_services.proto"),
        security_dir.join("security_services.proto"),
        runtime_dir.join("runtime_services.proto"),
        capability_dir.join("capability_services.proto"),
        agent_dir.join("agent_services.proto"),
        agent_dir.join("agent_runtime_services.proto"),
    ];

    println!(
        "cargo:rerun-if-changed={}",
        source_root.join("messages/common.proto").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        source_root.join("messages/bus.proto").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        source_root.join("services/core_services.proto").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        source_root.join("services/ai_services.proto").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        source_root
            .join("services/security_services.proto")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        source_root
            .join("services/runtime_services.proto")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        source_root
            .join("services/capability_services.proto")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        source_root.join("services/agent_services.proto").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        source_root.join("services/agent_runtime_services.proto").display()
    );

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&proto_files, &[&proto_root])?;

    Ok(())
}
