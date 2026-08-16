#![cfg(target_os = "windows")]

use crate::model::*;
use crate::provider::{CapabilityProvider, CapabilityProviderContext, CapabilityProviderResult};
use async_trait::async_trait;
use image::RgbaImage;
use serde_json::{json, Value};
use tokio::task::spawn_blocking;
use uuid::Uuid;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

fn err(code: CapabilityErrorCode, message: impl Into<String>) -> CapabilityError {
    CapabilityError {
        code,
        message: message.into(),
        retryable: false,
    }
}

pub(crate) fn force_foreground_window(hwnd: HWND) {
    unsafe {
        let foreground = GetForegroundWindow();
        let mut foreground_tid_pid = 0u32;
        let foreground_tid = GetWindowThreadProcessId(foreground, Some(&mut foreground_tid_pid));
        let mut target_tid_pid = 0u32;
        let target_tid = GetWindowThreadProcessId(hwnd, Some(&mut target_tid_pid));
        let current_tid = GetCurrentThreadId();
        let _ = AttachThreadInput(current_tid, foreground_tid, true);
        if target_tid != 0 && target_tid != current_tid {
            let _ = AttachThreadInput(current_tid, target_tid, true);
        }
        let _ = LockSetForegroundWindow(LSFW_UNLOCK);
        let _ = AllowSetForegroundWindow(ASFW_ANY);
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = BringWindowToTop(hwnd);
        if crate::gui_test::enabled() {
            let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        }
        let mut alt = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_MENU,
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        SendInput(&[alt], std::mem::size_of::<INPUT>() as i32);
        let _ = SetForegroundWindow(hwnd);
        alt.Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
        SendInput(&[alt], std::mem::size_of::<INPUT>() as i32);
        if crate::gui_test::enabled() {
            let mut client = RECT::default();
            if GetClientRect(hwnd, &mut client).is_ok() {
                let mut point = POINT {
                    x: client.right / 2,
                    y: client.bottom.max(1) / 3,
                };
                let _ = ClientToScreen(hwnd, &mut point);
                let _ = SetCursorPos(point.x, point.y);
                let down = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: 0,
                            dwFlags: MOUSEEVENTF_LEFTDOWN,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                let mut up = down;
                up.Anonymous.mi.dwFlags = MOUSEEVENTF_LEFTUP;
                SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
            }
        }
        if target_tid != 0 && target_tid != current_tid {
            let _ = AttachThreadInput(current_tid, target_tid, false);
        }
        let _ = AttachThreadInput(current_tid, foreground_tid, false);
    }
}

fn hwnd_from_window_id(window_id: &str) -> Result<HWND, CapabilityError> {
    let ptr_val = window_id
        .strip_prefix("hwnd:")
        .and_then(|s| s.parse::<usize>().ok())
        .ok_or_else(|| {
            err(
                CapabilityErrorCode::InvalidInput,
                "Invalid window_id format",
            )
        })?;
    Ok(HWND(ptr_val as _))
}

fn hwnds_equal(left: HWND, right: HWND) -> bool {
    left.0 == right.0
}

fn foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

fn window_is_focused(hwnd: HWND) -> bool {
    let foreground = foreground_window();
    if hwnds_equal(foreground, hwnd) {
        return true;
    }
    unsafe {
        let root = GetAncestor(foreground, GA_ROOT);
        if hwnds_equal(root, hwnd) {
            return true;
        }
        let mut current = foreground;
        for _ in 0..8 {
            let Ok(parent) = GetParent(current) else {
                break;
            };
            if parent.0 as usize == 0 {
                break;
            }
            if hwnds_equal(parent, hwnd) {
                return true;
            }
            current = parent;
        }
    }
    false
}

fn read_document_text(hwnd: HWND) -> String {
    unsafe {
        for class in ["Edit", "RichEdit20W", "RICHEDIT50W"] {
            let class_w: Vec<u16> = class.encode_utf16().chain(std::iter::once(0)).collect();
            if let Ok(child) = FindWindowExW(
                hwnd,
                HWND::default(),
                windows::core::PCWSTR(class_w.as_ptr()),
                windows::core::PCWSTR::null(),
            ) {
                if child.0 as usize != 0 {
                    let text = control_text(child);
                    if !text.is_empty() {
                        return text;
                    }
                }
            }
        }
        read_document_text_uia(hwnd)
    }
}

fn read_document_text_uia(hwnd: HWND) -> String {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTextPattern,
        TreeScope_Descendants, UIA_TextPatternId, UIA_ValueValuePropertyId,
    };
    fn element_text(element: &IUIAutomationElement) -> String {
        unsafe {
            if let Ok(pattern) =
                element.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
            {
                if let Ok(range) = pattern.DocumentRange() {
                    if let Ok(text) = range.GetText(-1) {
                        let text = text.to_string();
                        if !text.is_empty() {
                            return text;
                        }
                    }
                }
            }
            if let Ok(value) = element.GetCurrentPropertyValue(UIA_ValueValuePropertyId) {
                if let Ok(text) = windows::core::BSTR::try_from(&value) {
                    return text.to_string();
                }
            }
            String::new()
        }
    }
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Ok(automation) = CoCreateInstance::<Option<&windows::core::IUnknown>, IUIAutomation>(
            &CUIAutomation,
            None,
            CLSCTX_ALL,
        ) else {
            return String::new();
        };
        let Ok(root) = automation.ElementFromHandle(hwnd) else {
            return String::new();
        };
        let mut title = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title);
        let window_title = if title_len > 0 {
            String::from_utf16_lossy(&title[..title_len as usize])
        } else {
            String::new()
        };
        let title_echo = window_title
            .trim_start_matches(['*', ' '])
            .split(" - ")
            .next()
            .unwrap_or(window_title.trim())
            .trim()
            .to_ascii_lowercase();
        let mut candidates = Vec::new();
        let mut best = String::new();
        let mut consider = |text: String| {
            let trimmed = text.trim().to_string();
            if trimmed.is_empty() {
                return;
            }
            let lower = trimmed.to_ascii_lowercase();
            if lower == title_echo
                || lower == window_title.to_ascii_lowercase()
                || lower.contains("crlf")
                || lower.contains("utf-")
                || lower.contains("character")
                || lower == "plain text"
                || lower.starts_with("ln ")
                || lower.starts_with("col ")
            {
                return;
            }
            if !candidates.contains(&trimmed) {
                candidates.push(trimmed.clone());
            }
            if trimmed.len() > best.len() {
                best = trimmed;
            }
        };
        consider(element_text(&root));
        if let Ok(focused) = automation.GetFocusedElement() {
            consider(element_text(&focused));
        }
        if let Ok(condition) = automation.CreateTrueCondition() {
            if let Ok(array) = root.FindAll(TreeScope_Descendants, &condition) {
                if let Ok(len) = array.Length() {
                    for index in 0..len.min(400) {
                        if let Ok(element) = array.GetElement(index) {
                            consider(element_text(&element));
                        }
                    }
                }
            }
        }
        if crate::gui_test::enabled() && !candidates.is_empty() {
            candidates.join("\n")
        } else {
            best
        }
    }
}

fn control_text(hwnd: HWND) -> String {
    unsafe {
        let length = SendMessageW(hwnd, WM_GETTEXTLENGTH, WPARAM(0), LPARAM(0)).0;
        if length > 0 {
            let mut buf = vec![0u16; length as usize + 1];
            let copied = SendMessageW(
                hwnd,
                WM_GETTEXT,
                WPARAM(buf.len()),
                LPARAM(buf.as_mut_ptr() as isize),
            )
            .0;
            if copied > 0 {
                return String::from_utf16_lossy(&buf[..copied as usize]);
            }
        }
        let mut buf = [0u16; 4096];
        let len = GetWindowTextW(hwnd, &mut buf);
        if len > 0 {
            String::from_utf16_lossy(&buf[..len as usize])
        } else {
            String::new()
        }
    }
}

fn focus_window_id(window_id: &str) -> Result<HWND, CapabilityError> {
    let hwnd = hwnd_from_window_id(window_id)?;
    for _ in 0..8 {
        force_foreground_window(hwnd);
        std::thread::sleep(std::time::Duration::from_millis(80));
        if window_is_focused(hwnd) {
            return Ok(hwnd);
        }
    }
    if crate::gui_test::enabled() {
        return Err(err(
            CapabilityErrorCode::InvalidInput,
            "TEST_TARGET_UNSAFE: focus could not be verified before keyboard input",
        ));
    }
    Ok(hwnd)
}

fn send_vk(vk: VIRTUAL_KEY, up: bool) {
    unsafe {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up {
                        KEYEVENTF_KEYUP
                    } else {
                        KEYBD_EVENT_FLAGS(0)
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

fn type_into_document(hwnd: HWND, text: &str) -> bool {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationValuePattern, TreeScope_Descendants,
        UIA_ControlTypePropertyId, UIA_DocumentControlTypeId, UIA_ValuePatternId,
    };
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Ok(automation) = CoCreateInstance::<Option<&windows::core::IUnknown>, IUIAutomation>(
            &CUIAutomation,
            None,
            CLSCTX_ALL,
        ) else {
            type_text_virtual_keys(text);
            return false;
        };
        let Ok(root) = automation.ElementFromHandle(hwnd) else {
            type_text_virtual_keys(text);
            return false;
        };
        for control in [UIA_DocumentControlTypeId.0] {
            let Ok(condition) = automation.CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &windows::core::VARIANT::from(control),
            ) else {
                continue;
            };
            let Ok(array) = root.FindAll(TreeScope_Descendants, &condition) else {
                continue;
            };
            let Ok(len) = array.Length() else {
                continue;
            };
            for index in 0..len {
                let Ok(element) = array.GetElement(index) else {
                    continue;
                };
                let _ = element.SetFocus();
                if let Ok(pattern) =
                    element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
                {
                    if pattern.SetValue(&windows::core::BSTR::from(text)).is_ok() {
                        return true;
                    }
                }
                type_text_virtual_keys(text);
                return true;
            }
        }
        type_text_virtual_keys(text);
        false
    }
}

fn type_text_virtual_keys(text: &str) {
    unsafe {
        for c in text.chars() {
            let scan = VkKeyScanW(c as u16);
            if scan == -1 {
                continue;
            }
            let vk = VIRTUAL_KEY((scan & 0xFF) as u16);
            let shift = (scan & 0x100) != 0;
            if shift {
                send_vk(VK_SHIFT, false);
            }
            send_vk(vk, false);
            send_vk(vk, true);
            if shift {
                send_vk(VK_SHIFT, true);
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
    }
}

fn focus_requested_window(input: &Value) -> Result<(), CapabilityError> {
    if let Some(window_id) = input.get("window_id").and_then(Value::as_str) {
        if !window_id.is_empty() {
            focus_window_id(window_id)?;
        }
    }
    Ok(())
}

pub struct WindowsScreenCaptureProvider {
    provider_id: String,
    runtime_id: String,
}

impl WindowsScreenCaptureProvider {
    pub fn new(provider_id: impl Into<String>, runtime_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            runtime_id: runtime_id.into(),
        }
    }
}

#[async_trait]
impl CapabilityProvider for WindowsScreenCaptureProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    fn definitions(&self) -> Vec<CapabilityDefinition> {
        let mut def = CapabilityDefinition::basic(
            "screen.capture",
            "Capture the primary monitor",
            vec![CapabilityRuntime::Windows],
            Idempotency::ReadOnly,
        );
        def.metadata.security_level = SecurityLevel::Sensitive;
        def.metadata.required_permissions = vec!["screen.capture".to_string()];
        vec![def]
    }

    async fn execute(
        &self,
        context: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError> {
        if context.request.capability_id != "screen.capture" {
            return Err(err(
                CapabilityErrorCode::Unsupported,
                "Unsupported capability",
            ));
        }

        let result = spawn_blocking(move || unsafe {
            let hdc_screen = GetDC(HWND::default());
            if hdc_screen.is_invalid() {
                return Err(err(CapabilityErrorCode::Internal, "Failed to GetDC"));
            }
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_invalid() {
                ReleaseDC(HWND::default(), hdc_screen);
                return Err(err(
                    CapabilityErrorCode::Internal,
                    "Failed to CreateCompatibleDC",
                ));
            }

            let width = GetSystemMetrics(SM_CXSCREEN);
            let height = GetSystemMetrics(SM_CYSCREEN);

            let hbm = CreateCompatibleBitmap(hdc_screen, width, height);
            if hbm.is_invalid() {
                let _ = DeleteDC(hdc_mem);
                ReleaseDC(HWND::default(), hdc_screen);
                return Err(err(
                    CapabilityErrorCode::Internal,
                    "Failed to CreateCompatibleBitmap",
                ));
            }

            SelectObject(hdc_mem, hbm);

            if BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY).is_err() {
                let _ = DeleteObject(hbm);
                let _ = DeleteDC(hdc_mem);
                ReleaseDC(HWND::default(), hdc_screen);
                return Err(err(CapabilityErrorCode::Internal, "BitBlt failed"));
            }

            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width;
            bmi.bmiHeader.biHeight = -height;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB.0;

            let mut pixels = vec![0u8; (width * height * 4) as usize];

            let scan_lines = GetDIBits(
                hdc_screen,
                hbm,
                0,
                height as u32,
                Some(pixels.as_mut_ptr() as *mut std::ffi::c_void),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            let _ = DeleteObject(hbm);
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(HWND::default(), hdc_screen);

            if scan_lines == 0 {
                return Err(err(CapabilityErrorCode::Internal, "GetDIBits failed"));
            }

            for chunk in pixels.chunks_exact_mut(4) {
                let b = chunk[0];
                let r = chunk[2];
                chunk[0] = r;
                chunk[2] = b;
                chunk[3] = 255;
            }

            let img = match RgbaImage::from_raw(width as u32, height as u32, pixels) {
                Some(i) => i,
                None => {
                    return Err(err(
                        CapabilityErrorCode::Internal,
                        "Image conversion failed",
                    ))
                }
            };

            let mut buf = std::io::Cursor::new(Vec::new());
            if img.write_to(&mut buf, image::ImageFormat::Png).is_err() {
                return Err(err(CapabilityErrorCode::Internal, "PNG encode failed"));
            }

            use base64::{engine::general_purpose, Engine as _};
            let b64 = general_purpose::STANDARD.encode(buf.into_inner());

            Ok((b64, width, height))
        })
        .await
        .map_err(|_| err(CapabilityErrorCode::Internal, "Thread pool error"))??;

        let (b64, w, h) = result;
        let artifact_id = Uuid::now_v7().to_string();

        Ok(CapabilityProviderResult {
            output: json!({
                "image_b64": b64,
                "width": w,
                "height": h,
                "format": "png",
                "timestamp_ms": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
                "artifact_id": artifact_id,
            }),
            artifacts: vec![artifact_id],
            side_effects: vec![],
            metadata: json!({"native": true, "api": "Win32", "host_os": "windows"}),
        })
    }
}

pub struct WindowsWindowProvider {
    provider_id: String,
    runtime_id: String,
}

impl WindowsWindowProvider {
    pub fn new(provider_id: impl Into<String>, runtime_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            runtime_id: runtime_id.into(),
        }
    }
}

#[async_trait]
impl CapabilityProvider for WindowsWindowProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    fn definitions(&self) -> Vec<CapabilityDefinition> {
        let caps = vec![
            "window.list",
            "window.inspect",
            "window.focus",
            "window.close",
            "window.minimize",
            "window.maximize",
            "window.move",
            "window.resize",
            "window.activate",
        ];

        caps.into_iter()
            .map(|cap| {
                let mut def = CapabilityDefinition::basic(
                    cap,
                    "Window management operation",
                    vec![CapabilityRuntime::Windows],
                    if cap.starts_with("window.list") || cap.starts_with("window.inspect") {
                        Idempotency::ReadOnly
                    } else {
                        Idempotency::NonIdempotent
                    },
                );
                def.metadata.security_level = if cap == "window.list" || cap == "window.inspect" {
                    SecurityLevel::Low
                } else {
                    SecurityLevel::Sensitive
                };
                def.metadata.required_permissions = vec![cap.to_string()];
                def
            })
            .collect()
    }

    async fn execute(
        &self,
        context: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError> {
        let req = context.request;
        let input = req.input.clone();
        let cap = req.capability_id.clone();

        let output =
            spawn_blocking(move || -> Result<Value, CapabilityError> {
                unsafe {
                    match cap.as_str() {
                        "window.list" => {
                            let mut windows: Vec<Value> = Vec::new();
                            let ptr = LPARAM(&mut windows as *mut Vec<Value> as isize);

                            unsafe extern "system" fn enum_windows_proc(
                                hwnd: HWND,
                                lparam: LPARAM,
                            ) -> BOOL {
                                if IsWindowVisible(hwnd).as_bool() {
                                    let mut title = [0u16; 512];
                                    let len = GetWindowTextW(hwnd, &mut title);
                                    if len > 0 {
                                        let title_str =
                                            String::from_utf16_lossy(&title[..len as usize]);

                                        let mut class_name = [0u16; 512];
                                        let class_len = GetClassNameW(hwnd, &mut class_name);
                                        let class_str = if class_len > 0 {
                                            String::from_utf16_lossy(
                                                &class_name[..class_len as usize],
                                            )
                                        } else {
                                            String::new()
                                        };

                                        let mut pid = 0;
                                        GetWindowThreadProcessId(hwnd, Some(&mut pid));

                                        let mut rect: windows::Win32::Foundation::RECT =
                                            std::mem::zeroed();
                                        let _ = GetWindowRect(hwnd, &mut rect);

                                        let minimized = IsIconic(hwnd).as_bool();

                                        let windows_vec = &mut *(lparam.0 as *mut std::ffi::c_void
                                            as *mut Vec<Value>);
                                        windows_vec.push(json!({
                                            "window_id": format!("hwnd:{}", hwnd.0 as usize),
                                            "title": title_str,
                                            "class_name": class_str,
                                            "process_id": pid,
                                            "bounds": {
                                                "x": rect.left,
                                                "y": rect.top,
                                                "width": rect.right - rect.left,
                                                "height": rect.bottom - rect.top
                                            },
                                            "visible": true,
                                            "minimized": minimized
                                        }));
                                    }
                                }
                                BOOL(1)
                            }

                            let _ = EnumWindows(Some(enum_windows_proc), ptr);
                            Ok(json!(windows))
                        }
                        "window.inspect" => {
                            let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(
                                || err(CapabilityErrorCode::InvalidInput, "Missing window_id"),
                            )?;
                            let ptr_val = wid
                                .strip_prefix("hwnd:")
                                .and_then(|s| s.parse::<usize>().ok())
                                .ok_or_else(|| {
                                    err(
                                        CapabilityErrorCode::InvalidInput,
                                        "Invalid window_id format",
                                    )
                                })?;
                            let hwnd = HWND(ptr_val as _);

                            let mut title = [0u16; 512];
                            let len = GetWindowTextW(hwnd, &mut title);
                            let title_str = if len > 0 {
                                String::from_utf16_lossy(&title[..len as usize])
                            } else {
                                String::new()
                            };

                            let mut class_name = [0u16; 512];
                            let class_len = GetClassNameW(hwnd, &mut class_name);
                            let class_str = if class_len > 0 {
                                String::from_utf16_lossy(&class_name[..class_len as usize])
                            } else {
                                String::new()
                            };

                            let mut pid = 0;
                            GetWindowThreadProcessId(hwnd, Some(&mut pid));

                            let mut rect: windows::Win32::Foundation::RECT = std::mem::zeroed();
                            let _ = GetWindowRect(hwnd, &mut rect);

                            let visible = IsWindowVisible(hwnd).as_bool();
                            let minimized = IsIconic(hwnd).as_bool();
                            let focused = window_is_focused(hwnd);
                            let text = read_document_text(hwnd);

                            Ok(json!({
                                "window_id": wid,
                                "title": title_str,
                                "class_name": class_str,
                                "process_id": pid,
                                "focused": focused,
                                "text": text,
                                "bounds": {
                                    "x": rect.left,
                                    "y": rect.top,
                                    "width": rect.right - rect.left,
                                    "height": rect.bottom - rect.top
                                },
                                "visible": visible,
                                "minimized": minimized
                            }))
                        }
                        "window.focus" | "window.activate" => {
                            let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(
                                || err(CapabilityErrorCode::InvalidInput, "Missing window_id"),
                            )?;
                            let hwnd = hwnd_from_window_id(wid)?;
                            force_foreground_window(hwnd);
                            let focused = window_is_focused(hwnd);
                            Ok(json!({
                                "success": true,
                                "focused": focused,
                                "window_id": wid,
                            }))
                        }
                        "window.close" => {
                            let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(
                                || err(CapabilityErrorCode::InvalidInput, "Missing window_id"),
                            )?;
                            let ptr_val = wid
                                .strip_prefix("hwnd:")
                                .and_then(|s| s.parse::<usize>().ok())
                                .ok_or_else(|| {
                                    err(
                                        CapabilityErrorCode::InvalidInput,
                                        "Invalid window_id format",
                                    )
                                })?;
                            let hwnd = HWND(ptr_val as _);
                            let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
                            Ok(json!({"success": true}))
                        }
                        "window.minimize" => {
                            let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(
                                || err(CapabilityErrorCode::InvalidInput, "Missing window_id"),
                            )?;
                            let ptr_val = wid
                                .strip_prefix("hwnd:")
                                .and_then(|s| s.parse::<usize>().ok())
                                .ok_or_else(|| {
                                    err(
                                        CapabilityErrorCode::InvalidInput,
                                        "Invalid window_id format",
                                    )
                                })?;
                            let hwnd = HWND(ptr_val as _);
                            let _ = ShowWindow(hwnd, SW_MINIMIZE);
                            Ok(json!({"success": true}))
                        }
                        "window.maximize" => {
                            let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(
                                || err(CapabilityErrorCode::InvalidInput, "Missing window_id"),
                            )?;
                            let ptr_val = wid
                                .strip_prefix("hwnd:")
                                .and_then(|s| s.parse::<usize>().ok())
                                .ok_or_else(|| {
                                    err(
                                        CapabilityErrorCode::InvalidInput,
                                        "Invalid window_id format",
                                    )
                                })?;
                            let hwnd = HWND(ptr_val as _);
                            let _ = ShowWindow(hwnd, SW_MAXIMIZE);
                            Ok(json!({"success": true}))
                        }
                        "window.move" => {
                            let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(
                                || err(CapabilityErrorCode::InvalidInput, "Missing window_id"),
                            )?;
                            let ptr_val = wid
                                .strip_prefix("hwnd:")
                                .and_then(|s| s.parse::<usize>().ok())
                                .ok_or_else(|| {
                                    err(
                                        CapabilityErrorCode::InvalidInput,
                                        "Invalid window_id format",
                                    )
                                })?;
                            let hwnd = HWND(ptr_val as _);
                            let x = input.get("x").and_then(|v| v.as_i64()).ok_or_else(|| {
                                err(CapabilityErrorCode::InvalidInput, "Missing x")
                            })? as i32;
                            let y = input.get("y").and_then(|v| v.as_i64()).ok_or_else(|| {
                                err(CapabilityErrorCode::InvalidInput, "Missing y")
                            })? as i32;
                            let _ = SetWindowPos(
                                hwnd,
                                HWND::default(),
                                x,
                                y,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOZORDER,
                            );
                            Ok(json!({"success": true}))
                        }
                        "window.resize" => {
                            let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(
                                || err(CapabilityErrorCode::InvalidInput, "Missing window_id"),
                            )?;
                            let ptr_val = wid
                                .strip_prefix("hwnd:")
                                .and_then(|s| s.parse::<usize>().ok())
                                .ok_or_else(|| {
                                    err(
                                        CapabilityErrorCode::InvalidInput,
                                        "Invalid window_id format",
                                    )
                                })?;
                            let hwnd = HWND(ptr_val as _);
                            let w =
                                input.get("width").and_then(|v| v.as_i64()).ok_or_else(|| {
                                    err(CapabilityErrorCode::InvalidInput, "Missing width")
                                })? as i32;
                            let h =
                                input
                                    .get("height")
                                    .and_then(|v| v.as_i64())
                                    .ok_or_else(|| {
                                        err(CapabilityErrorCode::InvalidInput, "Missing height")
                                    })? as i32;
                            let _ = SetWindowPos(
                                hwnd,
                                HWND::default(),
                                0,
                                0,
                                w,
                                h,
                                SWP_NOMOVE | SWP_NOZORDER,
                            );
                            Ok(json!({"success": true}))
                        }
                        _ => Err(err(
                            CapabilityErrorCode::Unsupported,
                            "Unsupported capability",
                        )),
                    }
                }
            })
            .await
            .map_err(|_| err(CapabilityErrorCode::Internal, "Thread pool error"))??;

        Ok(CapabilityProviderResult {
            output,
            artifacts: vec![],
            side_effects: vec![],
            metadata: json!({"native": true, "api": "Win32", "host_os": "windows"}),
        })
    }
}

pub struct WindowsKeyboardProvider {
    provider_id: String,
    runtime_id: String,
}

impl WindowsKeyboardProvider {
    pub fn new(provider_id: impl Into<String>, runtime_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            runtime_id: runtime_id.into(),
        }
    }
}

#[async_trait]
impl CapabilityProvider for WindowsKeyboardProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    fn definitions(&self) -> Vec<CapabilityDefinition> {
        let caps = vec!["keyboard.type", "keyboard.press", "keyboard.hotkey"];
        caps.into_iter()
            .map(|cap| {
                let mut def = CapabilityDefinition::basic(
                    cap,
                    "Keyboard operation",
                    vec![CapabilityRuntime::Windows],
                    Idempotency::NonIdempotent,
                );
                def.metadata.security_level = SecurityLevel::Sensitive;
                def.metadata.required_permissions = vec![cap.to_string()];
                def
            })
            .collect()
    }

    async fn execute(
        &self,
        context: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError> {
        let req = context.request;
        let input = req.input.clone();
        let cap = req.capability_id.clone();

        let output = spawn_blocking(move || -> Result<Value, CapabilityError> {
            unsafe {
                match cap.as_str() {
                    "keyboard.type" => {
                        let text = input.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                            err(CapabilityErrorCode::InvalidInput, "Missing text")
                        })?;
                        focus_requested_window(&input)?;
                        if crate::gui_test::enabled() {
                            if let Some(window_id) = input.get("window_id").and_then(Value::as_str)
                            {
                                if let Ok(hwnd) = hwnd_from_window_id(window_id) {
                                    type_into_document(hwnd, text);
                                } else {
                                    type_text_virtual_keys(text);
                                }
                            } else {
                                type_text_virtual_keys(text);
                            }
                        } else {
                            let mut inputs = Vec::new();
                            for c in text.encode_utf16() {
                                let mut ip = INPUT {
                                    r#type: INPUT_KEYBOARD,
                                    Anonymous: INPUT_0 {
                                        ki: KEYBDINPUT {
                                            wVk: VIRTUAL_KEY(0),
                                            wScan: c,
                                            dwFlags: KEYEVENTF_UNICODE,
                                            time: 0,
                                            dwExtraInfo: 0,
                                        },
                                    },
                                };
                                inputs.push(ip);
                                ip.Anonymous.ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
                                inputs.push(ip);
                            }
                            if !inputs.is_empty() {
                                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                            }
                        }
                        Ok(json!({"success": true}))
                    }
                    "keyboard.press" => {
                        let key = input
                            .get("key")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing key"))?;
                        let vk = match key.to_lowercase().as_str() {
                            "enter" => VK_RETURN,
                            "tab" => VK_TAB,
                            "escape" => VK_ESCAPE,
                            "space" => VK_SPACE,
                            "backspace" => VK_BACK,
                            "delete" => VK_DELETE,
                            "up" => VK_UP,
                            "down" => VK_DOWN,
                            "left" => VK_LEFT,
                            "right" => VK_RIGHT,
                            "home" => VK_HOME,
                            "end" => VK_END,
                            "pageup" => VK_PRIOR,
                            "pagedown" => VK_NEXT,
                            s if s.starts_with("f") && s.len() > 1 => {
                                let num = s[1..].parse::<u16>().unwrap_or(0);
                                if (1..=24).contains(&num) {
                                    VIRTUAL_KEY(VK_F1.0 + num - 1)
                                } else {
                                    return Err(err(
                                        CapabilityErrorCode::InvalidInput,
                                        "Invalid key",
                                    ));
                                }
                            }
                            s if s.len() == 1 => {
                                let c = s.chars().next().unwrap();
                                let k = VkKeyScanW(c as u16);
                                VIRTUAL_KEY((k & 0xFF) as u16)
                            }
                            _ => return Err(err(CapabilityErrorCode::InvalidInput, "Invalid key")),
                        };

                        let inputs = vec![
                            INPUT {
                                r#type: INPUT_KEYBOARD,
                                Anonymous: INPUT_0 {
                                    ki: KEYBDINPUT {
                                        wVk: vk,
                                        wScan: 0,
                                        dwFlags: KEYBD_EVENT_FLAGS(0),
                                        time: 0,
                                        dwExtraInfo: 0,
                                    },
                                },
                            },
                            INPUT {
                                r#type: INPUT_KEYBOARD,
                                Anonymous: INPUT_0 {
                                    ki: KEYBDINPUT {
                                        wVk: vk,
                                        wScan: 0,
                                        dwFlags: KEYEVENTF_KEYUP,
                                        time: 0,
                                        dwExtraInfo: 0,
                                    },
                                },
                            },
                        ];
                        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                        Ok(json!({"success": true}))
                    }
                    "keyboard.hotkey" => {
                        focus_requested_window(&input)?;
                        let keys =
                            input
                                .get("keys")
                                .and_then(|v| v.as_array())
                                .ok_or_else(|| {
                                    err(CapabilityErrorCode::InvalidInput, "Missing keys")
                                })?;
                        let mut vks = Vec::new();
                        for key_val in keys {
                            let key = key_val.as_str().ok_or_else(|| {
                                err(CapabilityErrorCode::InvalidInput, "Invalid key element")
                            })?;
                            let vk = match key.to_lowercase().as_str() {
                                "ctrl" => VK_CONTROL,
                                "alt" => VK_MENU,
                                "shift" => VK_SHIFT,
                                "win" => VK_LWIN,
                                s if s.len() == 1 => {
                                    let c = s.chars().next().unwrap();
                                    VIRTUAL_KEY((VkKeyScanW(c as u16) & 0xFF) as u16)
                                }
                                _ => {
                                    return Err(err(
                                        CapabilityErrorCode::InvalidInput,
                                        "Invalid key",
                                    ))
                                }
                            };
                            vks.push(vk);
                        }

                        let mut inputs = Vec::new();
                        for &vk in &vks {
                            inputs.push(INPUT {
                                r#type: INPUT_KEYBOARD,
                                Anonymous: INPUT_0 {
                                    ki: KEYBDINPUT {
                                        wVk: vk,
                                        wScan: 0,
                                        dwFlags: KEYBD_EVENT_FLAGS(0),
                                        time: 0,
                                        dwExtraInfo: 0,
                                    },
                                },
                            });
                        }
                        for &vk in vks.iter().rev() {
                            inputs.push(INPUT {
                                r#type: INPUT_KEYBOARD,
                                Anonymous: INPUT_0 {
                                    ki: KEYBDINPUT {
                                        wVk: vk,
                                        wScan: 0,
                                        dwFlags: KEYEVENTF_KEYUP,
                                        time: 0,
                                        dwExtraInfo: 0,
                                    },
                                },
                            });
                        }
                        if !inputs.is_empty() {
                            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                        }
                        Ok(json!({"success": true}))
                    }
                    _ => Err(err(
                        CapabilityErrorCode::Unsupported,
                        "Unsupported capability",
                    )),
                }
            }
        })
        .await
        .map_err(|_| err(CapabilityErrorCode::Internal, "Thread pool error"))??;

        Ok(CapabilityProviderResult {
            output,
            artifacts: vec![],
            side_effects: vec![],
            metadata: json!({"native": true, "api": "Win32", "host_os": "windows"}),
        })
    }
}

pub struct WindowsMouseProvider {
    provider_id: String,
    runtime_id: String,
}

impl WindowsMouseProvider {
    pub fn new(provider_id: impl Into<String>, runtime_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            runtime_id: runtime_id.into(),
        }
    }
}

#[async_trait]
impl CapabilityProvider for WindowsMouseProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    fn definitions(&self) -> Vec<CapabilityDefinition> {
        let caps = vec![
            "mouse.move",
            "mouse.click",
            "mouse.double_click",
            "mouse.right_click",
            "mouse.scroll",
        ];
        caps.into_iter()
            .map(|cap| {
                let mut def = CapabilityDefinition::basic(
                    cap,
                    "Mouse operation",
                    vec![CapabilityRuntime::Windows],
                    Idempotency::NonIdempotent,
                );
                def.metadata.security_level = SecurityLevel::Sensitive;
                def.metadata.required_permissions = vec![cap.to_string()];
                def
            })
            .collect()
    }

    async fn execute(
        &self,
        context: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError> {
        let req = context.request;
        let input = req.input.clone();
        let cap = req.capability_id.clone();

        let output = spawn_blocking(move || -> Result<Value, CapabilityError> {
            unsafe {
                let screen_w = GetSystemMetrics(SM_CXSCREEN) as f64;
                let screen_h = GetSystemMetrics(SM_CYSCREEN) as f64;

                match cap.as_str() {
                    "mouse.move" | "mouse.click" | "mouse.double_click" | "mouse.right_click" => {
                        let x = input
                            .get("x")
                            .and_then(|v| v.as_i64())
                            .ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing x"))?;
                        let y = input
                            .get("y")
                            .and_then(|v| v.as_i64())
                            .ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing y"))?;

                        let nx = ((x as f64 * 65535.0) / screen_w) as i32;
                        let ny = ((y as f64 * 65535.0) / screen_h) as i32;

                        let mut inputs = vec![INPUT {
                            r#type: INPUT_MOUSE,
                            Anonymous: INPUT_0 {
                                mi: MOUSEINPUT {
                                    dx: nx,
                                    dy: ny,
                                    mouseData: 0,
                                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                                    time: 0,
                                    dwExtraInfo: 0,
                                },
                            },
                        }];

                        if cap == "mouse.click" || cap == "mouse.double_click" {
                            inputs.push(INPUT {
                                r#type: INPUT_MOUSE,
                                Anonymous: INPUT_0 {
                                    mi: MOUSEINPUT {
                                        dx: nx,
                                        dy: ny,
                                        mouseData: 0,
                                        dwFlags: MOUSEEVENTF_LEFTDOWN | MOUSEEVENTF_ABSOLUTE,
                                        time: 0,
                                        dwExtraInfo: 0,
                                    },
                                },
                            });
                            inputs.push(INPUT {
                                r#type: INPUT_MOUSE,
                                Anonymous: INPUT_0 {
                                    mi: MOUSEINPUT {
                                        dx: nx,
                                        dy: ny,
                                        mouseData: 0,
                                        dwFlags: MOUSEEVENTF_LEFTUP | MOUSEEVENTF_ABSOLUTE,
                                        time: 0,
                                        dwExtraInfo: 0,
                                    },
                                },
                            });
                        }

                        if cap == "mouse.double_click" {
                            inputs.push(INPUT {
                                r#type: INPUT_MOUSE,
                                Anonymous: INPUT_0 {
                                    mi: MOUSEINPUT {
                                        dx: nx,
                                        dy: ny,
                                        mouseData: 0,
                                        dwFlags: MOUSEEVENTF_LEFTDOWN | MOUSEEVENTF_ABSOLUTE,
                                        time: 0,
                                        dwExtraInfo: 0,
                                    },
                                },
                            });
                            inputs.push(INPUT {
                                r#type: INPUT_MOUSE,
                                Anonymous: INPUT_0 {
                                    mi: MOUSEINPUT {
                                        dx: nx,
                                        dy: ny,
                                        mouseData: 0,
                                        dwFlags: MOUSEEVENTF_LEFTUP | MOUSEEVENTF_ABSOLUTE,
                                        time: 0,
                                        dwExtraInfo: 0,
                                    },
                                },
                            });
                        }

                        if cap == "mouse.right_click" {
                            inputs.push(INPUT {
                                r#type: INPUT_MOUSE,
                                Anonymous: INPUT_0 {
                                    mi: MOUSEINPUT {
                                        dx: nx,
                                        dy: ny,
                                        mouseData: 0,
                                        dwFlags: MOUSEEVENTF_RIGHTDOWN | MOUSEEVENTF_ABSOLUTE,
                                        time: 0,
                                        dwExtraInfo: 0,
                                    },
                                },
                            });
                            inputs.push(INPUT {
                                r#type: INPUT_MOUSE,
                                Anonymous: INPUT_0 {
                                    mi: MOUSEINPUT {
                                        dx: nx,
                                        dy: ny,
                                        mouseData: 0,
                                        dwFlags: MOUSEEVENTF_RIGHTUP | MOUSEEVENTF_ABSOLUTE,
                                        time: 0,
                                        dwExtraInfo: 0,
                                    },
                                },
                            });
                        }

                        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                        Ok(json!({"success": true}))
                    }
                    "mouse.scroll" => {
                        let delta =
                            input.get("delta").and_then(|v| v.as_i64()).ok_or_else(|| {
                                err(CapabilityErrorCode::InvalidInput, "Missing delta")
                            })? as i32;
                        let input_event = INPUT {
                            r#type: INPUT_MOUSE,
                            Anonymous: INPUT_0 {
                                mi: MOUSEINPUT {
                                    dx: 0,
                                    dy: 0,
                                    mouseData: (delta * 120) as u32,
                                    dwFlags: MOUSEEVENTF_WHEEL,
                                    time: 0,
                                    dwExtraInfo: 0,
                                },
                            },
                        };
                        SendInput(&[input_event], std::mem::size_of::<INPUT>() as i32);
                        Ok(json!({"success": true}))
                    }
                    _ => Err(err(
                        CapabilityErrorCode::Unsupported,
                        "Unsupported capability",
                    )),
                }
            }
        })
        .await
        .map_err(|_| err(CapabilityErrorCode::Internal, "Thread pool error"))??;

        Ok(CapabilityProviderResult {
            output,
            artifacts: vec![],
            side_effects: vec!["mouse.coordinate_input".to_string()],
            metadata: json!({"native": true, "api": "Win32", "host_os": "windows"}),
        })
    }
}
