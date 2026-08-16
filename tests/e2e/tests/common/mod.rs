//! Deterministic Windows GUI validation harness (Phase 13.5).
//!
//! Safety: never targets personal documents, never closes arbitrary Notepad
//! windows, and fails closed on ambiguity.

#![allow(dead_code)]

use cognyx_agent_core::PermissionContext;
use cognyx_capability::gui_test;
use cognyx_execution::native_host_runtime_id;
use cognyx_gateway::{CapabilityGateway, CapabilityRequest};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

pub const EXPECTED_TEXT: &str = "Hello CognyxOS";

pub struct OwnedTarget {
    pub window_id: String,
    pub title: String,
    pub process_id: Option<u64>,
}

pub struct GuiHarness {
    pub workspace: PathBuf,
    pub document: PathBuf,
    pub created_document: bool,
    pub owned: Option<OwnedTarget>,
    snapshot: Vec<String>,
}

impl GuiHarness {
    pub fn enable() {
        std::env::set_var(gui_test::GUI_TEST_ENV, "1");
        let document = gui_test::golden_document_path();
        std::env::set_var(
            gui_test::GUI_TEST_DOCUMENT_ENV,
            document.to_string_lossy().as_ref(),
        );
    }

    pub fn prepare() -> Result<Self, String> {
        Self::enable();
        let workspace = gui_test::test_workspace();
        std::fs::create_dir_all(&workspace).map_err(|error| error.to_string())?;
        let document = gui_test::ensure_golden_document()?;
        Ok(Self {
            workspace,
            document,
            created_document: true,
            owned: None,
            snapshot: Vec::new(),
        })
    }

    pub fn grants() -> HashSet<String> {
        HashSet::from([
            "application.search".into(),
            "application.open".into(),
            "keyboard.type".into(),
            "keyboard.hotkey".into(),
            "clipboard.read".into(),
            "clipboard.write".into(),
            "window.list".into(),
            "window.focus".into(),
            "window.inspect".into(),
            "window.close".into(),
        ])
    }

    pub fn ctx() -> PermissionContext {
        PermissionContext {
            user_id: "gui-test".into(),
            session_id: "gui-test".into(),
            granted_capabilities: Self::grants(),
            is_administrator: false,
        }
    }

    pub fn request(
        capability: &str,
        target: impl Into<String>,
        constraints: HashMap<String, String>,
    ) -> CapabilityRequest {
        CapabilityRequest {
            request_id: format!("gui-{}", uuid::Uuid::now_v7()),
            task_id: "gui-test".into(),
            agent_id: "gui-test".into(),
            capability: capability.into(),
            target: target.into(),
            arguments: vec![],
            constraints,
            permission_context: Self::ctx(),
            timeout_seconds: 20,
        }
    }

    pub async fn list_windows(gw: &CapabilityGateway) -> Result<Vec<Value>, String> {
        let listed = gw
            .execute_capability(Self::request("window.list", String::new(), HashMap::new()))
            .await;
        if !listed.success {
            return Err(listed.error.unwrap_or_else(|| "window.list failed".into()));
        }
        let parsed: Value = serde_json::from_str(&listed.output).unwrap_or(serde_json::json!([]));
        Ok(parsed
            .as_array()
            .cloned()
            .or_else(|| parsed.get("windows").and_then(Value::as_array).cloned())
            .unwrap_or_default())
    }

    pub async fn reject_leftover_golden_windows(
        &self,
        gw: &CapabilityGateway,
    ) -> Result<(), String> {
        let leftovers: Vec<_> = Self::list_windows(gw)
            .await?
            .into_iter()
            .filter(|window| {
                gui_test::is_test_owned_title(
                    window.get("title").and_then(Value::as_str).unwrap_or(""),
                )
            })
            .collect();
        if leftovers.is_empty() {
            return Ok(());
        }
        Err(format!(
            "TEST_TARGET_UNSAFE: leftover golden-test window {:?} requires manual close",
            leftovers[0].get("window_id")
        ))
    }

    pub async fn snapshot(&mut self, gw: &CapabilityGateway) -> Result<(), String> {
        self.snapshot = Self::list_windows(gw)
            .await?
            .iter()
            .filter_map(|window| {
                window
                    .get("window_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .collect();
        Ok(())
    }

    pub fn print_environment(&self, provider_id: &str, application: &str) {
        println!("TARGET ENVIRONMENT");
        println!("OS={}", std::env::consts::OS);
        println!("runtime_id={}", native_host_runtime_id());
        println!("provider_id={provider_id}");
        println!("test_workspace={}", self.workspace.display());
        println!("target application={application}");
        println!("test_document={}", self.document.display());
    }

    pub fn claim_from_open(&mut self, open_output: &str) -> Result<OwnedTarget, String> {
        let parsed: Value = serde_json::from_str(open_output)
            .map_err(|_| "TEST_TARGET_UNSAFE: application.open output is not JSON".to_string())?;
        let window_id = parsed
            .get("window_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                "TEST_TARGET_UNSAFE: application.open did not return window_id".to_string()
            })?
            .to_string();
        let title = parsed
            .get("window_title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if gui_test::is_protected_title(&title) {
            return Err(format!(
                "TEST_TARGET_UNSAFE: open targeted protected title '{title}'"
            ));
        }
        if !gui_test::is_test_owned_title(&title) {
            return Err(format!(
                "TEST_TARGET_UNSAFE: open title '{title}' is not the golden test document"
            ));
        }
        let owned = OwnedTarget {
            window_id,
            title,
            process_id: parsed.get("process_id").and_then(Value::as_u64),
        };
        self.owned = Some(OwnedTarget {
            window_id: owned.window_id.clone(),
            title: owned.title.clone(),
            process_id: owned.process_id,
        });
        Ok(owned)
    }

    pub async fn discover_owned(&mut self, gw: &CapabilityGateway) -> Result<OwnedTarget, String> {
        let windows = Self::list_windows(gw).await?;
        let mut candidates = Vec::new();
        for window in windows {
            let title = window
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if gui_test::is_protected_title(title) {
                continue;
            }
            if !gui_test::is_test_owned_title(title) {
                continue;
            }
            let window_id = window
                .get("window_id")
                .and_then(Value::as_str)
                .ok_or_else(|| "TEST_TARGET_UNSAFE: window is missing window_id".to_string())?;
            if self.snapshot.iter().any(|id| id == window_id) {
                continue;
            }
            candidates.push(OwnedTarget {
                window_id: window_id.to_string(),
                title: title.to_string(),
                process_id: window.get("process_id").and_then(Value::as_u64),
            });
        }
        if candidates.is_empty() {
            return Err(
                "TEST_TARGET_UNSAFE: no test-owned CognyxOS-Golden-Test window was found".into(),
            );
        }
        if candidates.len() > 1 {
            return Err(format!(
                "TEST_TARGET_UNSAFE: {} golden-test windows remain ambiguous",
                candidates.len()
            ));
        }
        let owned = candidates.remove(0);
        self.owned = Some(OwnedTarget {
            window_id: owned.window_id.clone(),
            title: owned.title.clone(),
            process_id: owned.process_id,
        });
        Ok(owned)
    }

    pub async fn verify_focus(
        &self,
        gw: &CapabilityGateway,
        window_id: &str,
    ) -> Result<(), String> {
        let focused = gw
            .execute_capability(Self::request("window.focus", window_id, HashMap::new()))
            .await;
        if !focused.success {
            return Err(format!(
                "TEST_TARGET_UNSAFE: focus failed: {:?}",
                focused.error
            ));
        }
        let parsed: Value = serde_json::from_str(&focused.output).unwrap_or_default();
        if parsed.get("focused").and_then(Value::as_bool) != Some(true) {
            return Err(
                "TEST_TARGET_UNSAFE: focus could not be verified for the owned window".into(),
            );
        }
        let inspected = gw
            .execute_capability(Self::request("window.inspect", window_id, HashMap::new()))
            .await;
        let inspect: Value = serde_json::from_str(&inspected.output).unwrap_or_default();
        let title = inspect
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if gui_test::is_protected_title(title) || !gui_test::is_test_owned_title(title) {
            return Err(format!(
                "TEST_TARGET_UNSAFE: focused window title '{title}' is not the test document"
            ));
        }
        Ok(())
    }

    pub async fn type_text(
        &self,
        gw: &CapabilityGateway,
        window_id: &str,
        text: &str,
    ) -> Result<(), String> {
        let typed = gw
            .execute_capability(Self::request(
                "keyboard.type",
                text,
                HashMap::from([("window_id".into(), window_id.to_string())]),
            ))
            .await;
        if !typed.success {
            return Err(format!("keyboard.type failed: {:?}", typed.error));
        }
        Ok(())
    }

    pub async fn verify_text(
        &self,
        gw: &CapabilityGateway,
        window_id: &str,
        expected: &str,
    ) -> Result<(), String> {
        self.verify_focus(gw, window_id).await?;
        let inspected = gw
            .execute_capability(Self::request("window.inspect", window_id, HashMap::new()))
            .await;
        let inspect: Value = serde_json::from_str(&inspected.output).unwrap_or_default();
        let document_text = inspect
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default();
        println!(
            "inspect title={:?} text={document_text:?}",
            inspect.get("title")
        );
        if document_text.contains(expected) {
            return Ok(());
        }
        match self.save_owned(gw, window_id).await {
            Ok(()) => return Ok(()),
            Err(error) => println!("owned-file verification: {error}"),
        }
        let previous = gw
            .execute_capability(Self::request(
                "clipboard.read",
                String::new(),
                HashMap::new(),
            ))
            .await;
        let previous_text = clipboard_payload(&previous.output);
        let window = HashMap::from([("window_id".into(), window_id.to_string())]);
        for hotkey in ["ctrl+a", "ctrl+c"] {
            let sent = gw
                .execute_capability(Self::request("keyboard.hotkey", hotkey, window.clone()))
                .await;
            if !sent.success {
                return Err(format!("{hotkey} failed: {:?}", sent.error));
            }
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
        let clip = gw
            .execute_capability(Self::request(
                "clipboard.read",
                String::new(),
                HashMap::new(),
            ))
            .await;
        let clip_text = clipboard_payload(&clip.output);
        let _ = gw
            .execute_capability(Self::request(
                "clipboard.write",
                previous_text,
                HashMap::new(),
            ))
            .await;
        if clip.success && clip_text.contains(expected) {
            return Ok(());
        }
        if let Ok(contents) = std::fs::read_to_string(&self.document) {
            if contents.contains(expected) {
                return Ok(());
            }
        }
        Err(format!(
            "TEXT_VERIFICATION failed: inspect_text={document_text:?} clipboard={clip_text:?} file_exists={}",
            self.document.exists()
        ))
    }

    pub async fn save_owned(&self, gw: &CapabilityGateway, window_id: &str) -> Result<(), String> {
        self.verify_focus(gw, window_id).await?;
        let saved = gw
            .execute_capability(Self::request(
                "keyboard.hotkey",
                "ctrl+s",
                HashMap::from([("window_id".into(), window_id.to_string())]),
            ))
            .await;
        if !saved.success {
            return Err(format!("save hotkey failed: {:?}", saved.error));
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
        let contents =
            std::fs::read_to_string(&self.document).map_err(|error| error.to_string())?;
        if !contents.contains(EXPECTED_TEXT) {
            return Err(format!(
                "TEST_TARGET_UNSAFE: saved test document did not contain {EXPECTED_TEXT}"
            ));
        }
        Ok(())
    }

    pub async fn cleanup(&self, gw: &CapabilityGateway) -> Result<(), String> {
        let Some(owned) = &self.owned else {
            println!("CLEANUP_REQUIRES_MANUAL_INTERVENTION: no owned window was recorded");
            return Ok(());
        };
        let windows = Self::list_windows(gw).await?;
        let still_ours = windows.iter().find(|window| {
            window.get("window_id").and_then(Value::as_str) == Some(owned.window_id.as_str())
        });
        let Some(window) = still_ours else {
            if self.created_document {
                let _ = std::fs::remove_file(&self.document);
            }
            return Ok(());
        };
        let title = window
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if gui_test::is_protected_title(title) || !gui_test::is_test_owned_title(title) {
            println!(
                "CLEANUP_REQUIRES_MANUAL_INTERVENTION: owned hwnd {} title '{title}' is no longer a proven test document",
                owned.window_id
            );
            return Ok(());
        }
        let _ = gw
            .execute_capability(Self::request(
                "window.close",
                &owned.window_id,
                HashMap::new(),
            ))
            .await;
        if self.created_document {
            let _ = std::fs::remove_file(&self.document);
        }
        Ok(())
    }
}

fn clipboard_payload(output: &str) -> String {
    serde_json::from_str::<Value>(output)
        .ok()
        .and_then(|value| {
            value
                .get("text")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| output.to_string())
}
