use cognyx_hardening::{
    first_boot_steps, BackupEngine, Doctor, Environment, HardeningError, Health, HealthStatus,
    ReleaseChannel, SecretStore, SystemConfig, UpdateManager,
};

#[test]
fn config_validation_and_channels() {
    let bad = SystemConfig {
        environment: Environment::Production,
        release_channel: ReleaseChannel::Nightly,
        version: "0.1.0".into(),
    };
    assert!(bad.validate().is_err());
    let ok = SystemConfig {
        environment: Environment::Production,
        release_channel: ReleaseChannel::Stable,
        version: "0.1.0".into(),
    };
    assert!(ok.validate().is_ok());
}

#[test]
fn secrets_never_enter_logs_or_backups() {
    let store = SecretStore::new();
    store.put("api", b"super-secret-token");
    assert!(matches!(
        store.redact("using super-secret-token in prompt"),
        Err(HardeningError::SecretLeak(_))
    ));
    let engine = BackupEngine::new(store);
    assert!(engine
        .backup("payload super-secret-token", vec!["workspace".into()])
        .is_err());
    let store2 = SecretStore::new();
    store2.put("api", b"super-secret-token");
    let engine2 = BackupEngine::new(store2);
    let bak = engine2
        .backup("workspace metadata only", vec!["workspace".into()])
        .unwrap();
    assert!(engine2.restore(&bak.id).is_ok());
}

#[test]
fn doctor_and_first_boot() {
    let report = Doctor::run();
    assert!(report.iter().any(|d| d.component == "security"));
    let virt = report
        .iter()
        .find(|d| d.component == "virtualization")
        .expect("virtualization row");
    if !virt.ok {
        assert_ne!(virt.status, HealthStatus::Healthy);
        assert_ne!(virt.status, HealthStatus::Available);
    }
    assert!(!first_boot_steps().is_empty());
    let _ = Health::all_ok();
}

#[test]
fn doctor_virtualization_never_ok_when_not_verified() {
    let report = Doctor::run();
    let virt = report
        .iter()
        .find(|d| d.component == "virtualization")
        .unwrap();
    if matches!(
        virt.status,
        HealthStatus::NotVerified
            | HealthStatus::Unavailable
            | HealthStatus::PermissionDenied
            | HealthStatus::NotInstalled
    ) {
        assert!(
            !virt.ok,
            "virtualization ok must be false when not verified: {:?}",
            virt
        );
    }
}

#[test]
fn update_rollback_does_not_leave_partial_state() {
    let mgr = UpdateManager::new("0.1.0");
    assert!(mgr.apply("0.2.0", false).is_err());
    assert_eq!(mgr.state().current, "0.1.0");
    mgr.apply("0.2.0", true).unwrap();
    let rolled = mgr.rollback().unwrap();
    assert_eq!(rolled.current, "0.1.0");
}
