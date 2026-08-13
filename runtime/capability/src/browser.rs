//! Universal browser automation provider.
//!
//! Uses Playwright (Node.js) as the real backend via subprocess.
//! If Playwright is not installed the provider returns RuntimeUnavailable \u2014
//! it never fakes a successful result.
use crate::model::*;
use crate::provider::{CapabilityProvider, CapabilityProviderContext, CapabilityProviderResult};
use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::{json, Value};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command;
use uuid::Uuid;

fn err(code: CapabilityErrorCode, message: impl Into<String>) -> CapabilityError {
    CapabilityError {
        code,
        message: message.into(),
        retryable: false,
    }
}

fn def(id: &str, description: &str, idempotency: Idempotency) -> CapabilityDefinition {
    let mut d = CapabilityDefinition::basic(
        id,
        description,
        vec![
            CapabilityRuntime::Windows,
            CapabilityRuntime::Linux,
            CapabilityRuntime::MacOS,
        ],
        idempotency,
    );
    d.metadata.required_permissions.push(id.into());
    d.metadata.security_level = SecurityLevel::Sensitive;
    d.metadata.risk_level = RiskLevel::Low;
    d
}

#[derive(Clone)]
struct BrowserSessionState {
    session_id: String,
    current_url: String,
    #[allow(dead_code)]
    created_at_ms: u64,
}

/// Cross-platform browser automation provider backed by Playwright (Node.js).
///
/// Each browser capability launches a headless Chromium instance via a short
/// Node.js script, executes the operation, returns JSON output, and closes.
/// Session state (current URL) is tracked in-memory so callers can issue
/// sequential `browser.*` calls against a logical session.
pub struct UniversalBrowserProvider {
    provider_id: String,
    runtime_id: String,
    sessions: Arc<DashMap<String, BrowserSessionState>>,
    temp_dir: PathBuf,
}

impl UniversalBrowserProvider {
    pub fn new(provider_id: impl Into<String>, runtime_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            runtime_id: runtime_id.into(),
            sessions: Arc::new(DashMap::new()),
            temp_dir: env::temp_dir().join("cognyxos_browser_scripts"),
        }
    }

    async fn playwright_available() -> bool {
        match Command::new("node")
            .arg("-e")
            .arg("require('playwright'); console.log('ok');")
            .output()
            .await
        {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }

    async fn ensure_temp_dir(&self) -> std::io::Result<()> {
        if !self.temp_dir.exists() {
            fs::create_dir_all(&self.temp_dir).await?;
        }
        Ok(())
    }

    async fn run_script(&self, script: &str) -> Result<Value, CapabilityError> {
        self.ensure_temp_dir()
            .await
            .map_err(|e| err(CapabilityErrorCode::Internal, format!("temp dir: {e}")))?;

        let path = self
            .temp_dir
            .join(format!("browser_{}.js", Uuid::now_v7()));
        fs::write(&path, script)
            .await
            .map_err(|e| err(CapabilityErrorCode::Internal, format!("write script: {e}")))?;

        let out = Command::new("node")
            .arg(&path)
            .output()
            .await
            .map_err(|e| {
                let _ = std::fs::remove_file(&path);
                err(CapabilityErrorCode::Internal, format!("node exec: {e}"))
            })?;
        let _ = fs::remove_file(&path).await;

        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        if !out.status.success() {
            return Err(err(
                CapabilityErrorCode::Internal,
                format!("node script failed: {stderr}"),
            ));
        }
        serde_json::from_str(&stdout)
            .map_err(|e| err(CapabilityErrorCode::Internal, format!("parse output: {e}")))
    }

    fn session_url(&self, session_id: &str) -> String {
        self.sessions
            .get(session_id)
            .map(|s| s.current_url.clone())
            .unwrap_or_else(|| "about:blank".into())
    }

    fn update_session_url(&self, session_id: &str, url: &str) {
        if let Some(mut s) = self.sessions.get_mut(session_id) {
            s.current_url = url.to_string();
        }
    }
}

#[async_trait]
impl CapabilityProvider for UniversalBrowserProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }
    fn priority(&self) -> u8 {
        5
    }
    fn definitions(&self) -> Vec<CapabilityDefinition> {
        vec![
            def(
                "browser.open",
                "Open a headless browser session and navigate to a URL",
                Idempotency::NonIdempotent,
            ),
            def(
                "browser.navigate",
                "Navigate the active session to a new URL",
                Idempotency::NonIdempotent,
            ),
            def(
                "browser.read",
                "Read the text content of the current page",
                Idempotency::ReadOnly,
            ),
            def(
                "browser.click",
                "Click a page element identified by CSS selector or text",
                Idempotency::NonIdempotent,
            ),
            def(
                "browser.type",
                "Type text into a page element identified by CSS selector",
                Idempotency::NonIdempotent,
            ),
            def(
                "browser.screenshot",
                "Capture the current page as a PNG screenshot",
                Idempotency::ReadOnly,
            ),
            def(
                "browser.close",
                "Close a browser session",
                Idempotency::Destructive,
            ),
            def(
                "browser.tabs",
                "List the tabs in the current session",
                Idempotency::ReadOnly,
            ),
            def(
                "browser.back",
                "Navigate back in browser history",
                Idempotency::NonIdempotent,
            ),
            def(
                "browser.forward",
                "Navigate forward in browser history",
                Idempotency::NonIdempotent,
            ),
            def(
                "browser.reload",
                "Reload the current page",
                Idempotency::NonIdempotent,
            ),
        ]
    }

    async fn execute(
        &self,
        context: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError> {
        let op = context.request.capability_id.as_str();
        let input = &context.request.input;

        // Check playwright before any operation that needs the browser
        let needs_playwright = !matches!(op, "browser.close" | "browser.tabs");
        if needs_playwright && !Self::playwright_available().await {
            return Err(err(
                CapabilityErrorCode::RuntimeUnavailable,
                "Playwright not found. Install: npm install -g playwright && npx playwright install chromium",
            ));
        }

        let output = match op {
            "browser.open" => {
                let url = input
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap_or("about:blank")
                    .to_string();
                let session_id = Uuid::now_v7().to_string();
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                self.sessions.insert(
                    session_id.clone(),
                    BrowserSessionState {
                        session_id: session_id.clone(),
                        current_url: url.clone(),
                        created_at_ms: now_ms,
                    },
                );

                let script = format!(
                    r#"const {{chromium}}=require('playwright');
(async()=>{{
  const b=await chromium.launch({{headless:true}});
  const p=await b.newPage();
  await p.goto('{url}',{{waitUntil:'domcontentloaded',timeout:30000}});
  const u=p.url();
  console.log(JSON.stringify({{session_id:'{session_id}',url:u}}));
  await b.close();
}})().catch(e=>{{console.error(JSON.stringify({{error:e.message}}));process.exit(1);}});"#
                );

                let result = self.run_script(&script).await?;
                if let Some(u) = result.get("url").and_then(Value::as_str) {
                    self.update_session_url(&session_id, u);
                }
                result
            }

            "browser.navigate" => {
                let session_id = input
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let url = input
                    .get("url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "input.url required"))?
                    .to_string();

                let script = format!(
                    r#"const {{chromium}}=require('playwright');
(async()=>{{
  const b=await chromium.launch({{headless:true}});
  const p=await b.newPage();
  await p.goto('{url}',{{waitUntil:'domcontentloaded',timeout:30000}});
  console.log(JSON.stringify({{url:p.url(),title:await p.title()}}));
  await b.close();
}})().catch(e=>{{console.error(JSON.stringify({{error:e.message}}));process.exit(1);}});"#
                );

                let result = self.run_script(&script).await?;
                if let Some(u) = result.get("url").and_then(Value::as_str) {
                    self.update_session_url(&session_id, u);
                }
                result
            }

            "browser.read" => {
                let session_id = input
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let url = self.session_url(session_id);

                let script = format!(
                    r#"const {{chromium}}=require('playwright');
(async()=>{{
  const b=await chromium.launch({{headless:true}});
  const p=await b.newPage();
  await p.goto('{url}',{{waitUntil:'domcontentloaded',timeout:30000}});
  const text=await p.innerText('body');
  console.log(JSON.stringify({{text,title:await p.title(),url:p.url()}}));
  await b.close();
}})().catch(e=>{{console.error(JSON.stringify({{error:e.message}}));process.exit(1);}});"#
                );
                self.run_script(&script).await?
            }

            "browser.click" => {
                let session_id = input
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let url = self.session_url(session_id);
                let selector = input
                    .get("selector")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .replace('\'', "\\'");
                let text_sel = input
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .replace('\'', "\\'");
                let click_expr = if !selector.is_empty() {
                    format!("page.locator('{selector}').click()")
                } else if !text_sel.is_empty() {
                    format!("page.getByText('{text_sel}').click()")
                } else {
                    "page.click('body')".into()
                };

                let script = format!(
                    r#"const {{chromium}}=require('playwright');
(async()=>{{
  const b=await chromium.launch({{headless:true}});
  const page=await b.newPage();
  await page.goto('{url}',{{waitUntil:'domcontentloaded',timeout:30000}});
  await {click_expr};
  await page.waitForLoadState('domcontentloaded');
  console.log(JSON.stringify({{clicked:true,url:page.url()}}));
  await b.close();
}})().catch(e=>{{console.error(JSON.stringify({{error:e.message}}));process.exit(1);}});"#
                );

                let result = self.run_script(&script).await?;
                if let Some(u) = result.get("url").and_then(Value::as_str) {
                    self.update_session_url(session_id, u);
                }
                json!({"clicked": true})
            }

            "browser.type" => {
                let session_id = input
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let url = self.session_url(session_id);
                let selector = input
                    .get("selector")
                    .and_then(Value::as_str)
                    .ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "input.selector required"))?
                    .replace('\'', "\\'");
                let text = input
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "input.text required"))?
                    .replace('\'', "\\'");

                let script = format!(
                    r#"const {{chromium}}=require('playwright');
(async()=>{{
  const b=await chromium.launch({{headless:true}});
  const p=await b.newPage();
  await p.goto('{url}',{{waitUntil:'domcontentloaded',timeout:30000}});
  await p.fill('{selector}','{text}');
  console.log(JSON.stringify({{typed:true}}));
  await b.close();
}})().catch(e=>{{console.error(JSON.stringify({{error:e.message}}));process.exit(1);}});"#
                );
                self.run_script(&script).await?
            }

            "browser.screenshot" => {
                let session_id = input
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let url = self.session_url(session_id);
                self.ensure_temp_dir().await.map_err(|e| {
                    err(CapabilityErrorCode::Internal, format!("temp dir: {e}"))
                })?;
                let shot_path = self
                    .temp_dir
                    .join(format!("shot_{}.png", Uuid::now_v7()));
                let shot_str = shot_path.to_string_lossy().replace('\\', "/");

                let script = format!(
                    r#"const {{chromium}}=require('playwright');
(async()=>{{
  const b=await chromium.launch({{headless:true}});
  const p=await b.newPage();
  await p.goto('{url}',{{waitUntil:'domcontentloaded',timeout:30000}});
  await p.screenshot({{path:'{shot_str}'}});
  console.log(JSON.stringify({{success:true}}));
  await b.close();
}})().catch(e=>{{console.error(JSON.stringify({{error:e.message}}));process.exit(1);}});"#
                );
                let _ = self.run_script(&script).await?;
                let bytes = fs::read(&shot_path).await.map_err(|e| {
                    err(CapabilityErrorCode::Internal, format!("read screenshot: {e}"))
                })?;
                let _ = fs::remove_file(&shot_path).await;
                use base64::{engine::general_purpose, Engine as _};
                let b64 = general_purpose::STANDARD.encode(&bytes);
                json!({"image_b64": b64, "format": "png"})
            }

            "browser.close" => {
                let session_id = input
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                self.sessions.remove(session_id);
                json!({"closed": true})
            }

            "browser.tabs" => {
                let session_id = input
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let url = self.session_url(session_id);
                json!({"tabs": [{"session_id": session_id, "url": url}]})
            }

            "browser.back" | "browser.forward" | "browser.reload" => {
                let session_id = input
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let url = self.session_url(session_id);
                let nav_action = match op {
                    "browser.back" => "p.goBack()",
                    "browser.forward" => "p.goForward()",
                    _ => "p.reload()",
                };
                let script = format!(
                    r#"const {{chromium}}=require('playwright');
(async()=>{{
  const b=await chromium.launch({{headless:true}});
  const p=await b.newPage();
  await p.goto('{url}',{{waitUntil:'domcontentloaded',timeout:30000}});
  await {nav_action};
  console.log(JSON.stringify({{url:p.url(),title:await p.title()}}));
  await b.close();
}})().catch(e=>{{console.error(JSON.stringify({{error:e.message}}));process.exit(1);}});"#
                );
                let result = self.run_script(&script).await?;
                if let Some(u) = result.get("url").and_then(Value::as_str) {
                    self.update_session_url(session_id, u);
                }
                result
            }

            _ => {
                return Err(err(
                    CapabilityErrorCode::Unsupported,
                    format!("unsupported browser operation '{op}'"),
                ))
            }
        };

        Ok(CapabilityProviderResult {
            output,
            artifacts: vec![],
            side_effects: vec![format!("{op}.executed")],
            metadata: json!({"native": false, "backend": "playwright-cli", "headless": true}),
        })
    }
}
