use cognyx_agent_memory::{
    ContextEngine, LongTermMemory, MemoryKind, MemoryPrivacy, ModelKind, ModelRouter, Reflection,
    WorkingMemory,
};
use std::collections::HashMap;
use std::sync::Arc;

fn privacy(owner: &str) -> MemoryPrivacy {
    MemoryPrivacy {
        owner: owner.into(),
        scope: "user".into(),
        retention_secs: 86400,
        visibility: "private".into(),
        classification: "general".into(),
        consent: true,
    }
}

fn ltm() -> LongTermMemory {
    let working = Arc::new(ContextEngine::new());
    working.update_working_memory(WorkingMemory {
        session_id: "sess-1".into(),
        current_task_id: Some("task-1".into()),
        current_plan_id: None,
        current_node_id: None,
        active_permissions: vec![],
        recent_results: vec![],
        working_variables: HashMap::new(),
    });
    LongTermMemory::new(working)
}

#[tokio::test]
async fn memory_create_retrieve_relevance() {
    let mem = ltm();
    mem.ingest(
        MemoryKind::Episodic,
        "opened the quarterly presentation in the documents folder",
        privacy("user"),
        Some("task-1".into()),
        None,
        Some("ws-1".into()),
    )
    .await
    .unwrap();
    mem.ingest(
        MemoryKind::Semantic,
        "user prefers vscode for rust",
        privacy("user"),
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let hits = mem.retrieve("presentation documents", "user", 4).await;
    assert!(!hits.is_empty());
}

#[tokio::test]
async fn privacy_and_real_deletion() {
    let mem = ltm();
    let rec = mem
        .ingest(
            MemoryKind::Task,
            "continue the report",
            privacy("user"),
            Some("task-9".into()),
            None,
            None,
        )
        .await
        .unwrap();
    assert!(mem.retrieve("report", "stranger", 4).await.is_empty());
    mem.delete(&rec.id, "user").unwrap();
    assert!(mem.view("user").iter().all(|r| r.id != rec.id));
}

#[tokio::test]
async fn refuse_sensitive_and_require_consent() {
    let mem = ltm();
    let mut secret = privacy("user");
    secret.classification = "secret".into();
    assert!(mem
        .ingest(
            MemoryKind::Episodic,
            "api key abc",
            secret,
            None,
            None,
            None
        )
        .await
        .is_err());
    let mut no = privacy("user");
    no.consent = false;
    assert!(mem
        .ingest(MemoryKind::Episodic, "ok", no, None, None, None)
        .await
        .is_err());
}

#[tokio::test]
async fn memory_disabled_mode() {
    let mem = ltm();
    mem.set_enabled(false);
    mem.ingest(
        MemoryKind::Episodic,
        "should not index",
        privacy("user"),
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert!(mem.retrieve("should not index", "user", 4).await.is_empty());
}

#[tokio::test]
async fn task_artifact_preference_and_working_untouched() {
    let mem = ltm();
    mem.ingest(
        MemoryKind::Artifact,
        "report.pdf",
        privacy("user"),
        None,
        Some("art-1".into()),
        None,
    )
    .await
    .unwrap();
    mem.remember_preference("user", "editor", "vscode");
    let wm = mem.working_memory("sess-1").unwrap();
    assert_eq!(wm.current_task_id.as_deref(), Some("task-1"));
    let n = mem.delete_category("user", MemoryKind::Preference);
    assert!(n >= 1);
}

#[tokio::test]
async fn consolidation_and_reflection() {
    let mem = ltm();
    mem.ingest(
        MemoryKind::ShortTerm,
        "long enough event to keep",
        privacy("user"),
        None,
        None,
        None,
    )
    .await
    .unwrap();
    mem.ingest(
        MemoryKind::ShortTerm,
        "x",
        privacy("user"),
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let n = mem.consolidate("user").await;
    assert!(n >= 1);
    let r = Reflection::from_task("task-1", false, "runtime timeout");
    assert!(!r.do_not_repeat.is_empty());
}

#[test]
fn model_routing_privacy_and_vision() {
    let local = ModelRouter::route(3, true, false, None, true);
    assert_eq!(local.kind, ModelKind::SmallLocal);
    let vision = ModelRouter::route(3, false, true, None, true);
    assert_eq!(vision.kind, ModelKind::Vision);
    let spec = ModelRouter::route(3, false, false, Some("code-specialist"), true);
    assert_eq!(spec.kind, ModelKind::Specialized);
}

#[tokio::test]
async fn context_limits() {
    let mem = ltm();
    for i in 0..20 {
        mem.ingest(
            MemoryKind::Episodic,
            format!("event number {i} about gardening"),
            privacy("user"),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    }
    let hits = mem.retrieve("gardening", "user", 3).await;
    assert!(hits.len() <= 8);
    assert!(hits.len() <= 3);
}
