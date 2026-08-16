use cognyx_plugin::{sample_echo_plugin, PluginError, PluginLifecycle, PluginRegistry};

#[test]
fn install_enable_execute_disable_capability_disappears() {
    let reg = PluginRegistry::new();
    let plugin = reg.install(sample_echo_plugin()).unwrap();
    reg.verify(&plugin.id).unwrap();
    reg.enable(&plugin.id).unwrap();
    assert!(reg.enabled_capabilities().contains(&"echo.say".to_string()));
    let out = reg
        .execute(
            &plugin.id,
            "echo.say",
            10,
            Some("/Workspace/Artifacts/x"),
            None,
        )
        .unwrap();
    assert!(out.contains("ok"));
    reg.disable(&plugin.id).unwrap();
    assert!(!reg.enabled_capabilities().contains(&"echo.say".to_string()));
    let err = reg
        .execute(&plugin.id, "echo.say", 10, None, None)
        .unwrap_err();
    assert!(matches!(err, PluginError::Disabled(_)));
}

#[test]
fn permissions_quotas_and_scopes() {
    let reg = PluginRegistry::new();
    let plugin = reg.install(sample_echo_plugin()).unwrap();
    reg.enable(&plugin.id).unwrap();
    assert!(matches!(
        reg.execute(&plugin.id, "filesystem.write", 10, None, None),
        Err(PluginError::PermissionDenied(_))
    ));
    assert!(matches!(
        reg.execute(&plugin.id, "echo.say", 9999, None, None),
        Err(PluginError::Quota(_))
    ));
    assert!(matches!(
        reg.execute(&plugin.id, "echo.say", 10, Some("/etc/passwd"), None),
        Err(PluginError::PermissionDenied(_))
    ));
    assert!(matches!(
        reg.execute(&plugin.id, "echo.say", 10, None, Some("evil.example")),
        Err(PluginError::PermissionDenied(_))
    ));
}

#[test]
fn upgrade_rollback_remove_and_audit() {
    let reg = PluginRegistry::new();
    let plugin = reg.install(sample_echo_plugin()).unwrap();
    reg.update(&plugin.id, "0.2.0").unwrap();
    let rolled = reg.rollback(&plugin.id).unwrap();
    assert_eq!(rolled.manifest.version, "0.1.0");
    assert_eq!(rolled.lifecycle, PluginLifecycle::RolledBack);
    reg.remove(&plugin.id).unwrap();
    assert!(reg.inspect(&plugin.id).is_err());
    let log = reg.audit_log();
    assert!(log.iter().any(|e| e.action == "install"));
    assert!(log.iter().any(|e| e.action == "rollback"));
    assert!(log.iter().any(|e| e.action == "remove"));
}

#[test]
fn plugin_cannot_inherit_user_terminal() {
    let reg = PluginRegistry::new();
    let mut m = sample_echo_plugin();
    m.permissions.push(cognyx_plugin::PluginPermission {
        name: "terminal.execute".into(),
    });
    assert!(matches!(
        reg.install(m),
        Err(PluginError::PermissionDenied(_))
    ));
}

#[test]
fn cli_surface() {
    assert_eq!(
        PluginRegistry::cli(&["install", "sample-echo"]),
        "cognyx plugin install sample-echo"
    );
}
