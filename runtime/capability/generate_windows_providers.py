import os

code = """#![cfg(target_os = "windows")]

use crate::model::*;
use crate::provider::{CapabilityProvider, CapabilityProviderContext, CapabilityProviderResult};
use async_trait::async_trait;
use image::{ImageBuffer, RgbImage, RgbaImage};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use uuid::Uuid;

use windows::core::Result as WinResult;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Foundation::CloseHandle;

fn err(code: CapabilityErrorCode, message: impl Into<String>) -> CapabilityError {
    CapabilityError {
        code,
        message: message.into(),
        retryable: false,
    }
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
    fn provider_id(&self) -> &str { &self.provider_id }
    fn runtime_id(&self) -> &str { &self.runtime_id }

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

    async fn execute(&self, context: CapabilityProviderContext) -> Result<CapabilityProviderResult, CapabilityError> {
        if context.request.capability_id != "screen.capture" {
            return Err(err(CapabilityErrorCode::Unsupported, "Unsupported capability"));
        }

        let provider_id = self.provider_id.clone();
        
        let result = spawn_blocking(move || {
            unsafe {
                let hdc_screen = GetDC(HWND(0));
                if hdc_screen.is_invalid() {
                    return Err(err(CapabilityErrorCode::Internal, "Failed to GetDC"));
                }
                let hdc_mem = CreateCompatibleDC(hdc_screen);
                if hdc_mem.is_invalid() {
                    ReleaseDC(HWND(0), hdc_screen);
                    return Err(err(CapabilityErrorCode::Internal, "Failed to CreateCompatibleDC"));
                }

                let width = GetSystemMetrics(SM_CXSCREEN);
                let height = GetSystemMetrics(SM_CYSCREEN);

                let hbm = CreateCompatibleBitmap(hdc_screen, width, height);
                if hbm.is_invalid() {
                    DeleteDC(hdc_mem);
                    ReleaseDC(HWND(0), hdc_screen);
                    return Err(err(CapabilityErrorCode::Internal, "Failed to CreateCompatibleBitmap"));
                }

                SelectObject(hdc_mem, hbm.into());

                let blt_res = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY);
                if let Err(_) = blt_res {
                    DeleteObject(hbm.into());
                    DeleteDC(hdc_mem);
                    ReleaseDC(HWND(0), hdc_screen);
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

                DeleteObject(hbm.into());
                DeleteDC(hdc_mem);
                ReleaseDC(HWND(0), hdc_screen);

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
                    None => return Err(err(CapabilityErrorCode::Internal, "Image conversion failed")),
                };

                let mut buf = std::io::Cursor::new(Vec::new());
                if let Err(_) = img.write_to(&mut buf, image::ImageFormat::Png) {
                    return Err(err(CapabilityErrorCode::Internal, "PNG encode failed"));
                }

                let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, buf.into_inner());

                Ok((b64, width, height))
            }
        }).await.map_err(|_| err(CapabilityErrorCode::Internal, "Thread pool error"))??;

        let (b64, w, h) = result;
        let artifact_id = Uuid::now_v7().to_string();

        Ok(CapabilityProviderResult {
            output: json!({
                "image_b64": b64,
                "width": w,
                "height": h,
                "format": "png",
                "timestamp_ms": crate::model::now_ms(),
                "provider_id": self.provider_id,
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
    fn provider_id(&self) -> &str { &self.provider_id }
    fn runtime_id(&self) -> &str { &self.runtime_id }

    fn definitions(&self) -> Vec<CapabilityDefinition> {
        let caps = vec![
            "window.list", "window.inspect", "window.focus", "window.close",
            "window.minimize", "window.maximize", "window.move", "window.resize", "window.activate"
        ];
        
        caps.into_iter().map(|cap| {
            let mut def = CapabilityDefinition::basic(
                cap,
                "Window management operation",
                vec![CapabilityRuntime::Windows],
                if cap.starts_with("window.list") || cap.starts_with("window.inspect") {
                    Idempotency::ReadOnly
                } else {
                    Idempotency::NonIdempotent
                }
            );
            def.metadata.security_level = if cap == "window.list" || cap == "window.inspect" {
                SecurityLevel::Low
            } else {
                SecurityLevel::Sensitive
            };
            def.metadata.required_permissions = vec![cap.to_string()];
            def
        }).collect()
    }

    async fn execute(&self, context: CapabilityProviderContext) -> Result<CapabilityProviderResult, CapabilityError> {
        let req = context.request;
        let input = req.input.clone();
        let cap = req.capability_id.clone();
        
        let output = spawn_blocking(move || -> Result<Value, CapabilityError> {
            unsafe {
                match cap.as_str() {
                    "window.list" => {
                        let mut windows: Vec<Value> = Vec::new();
                        let ptr = &mut windows as *mut Vec<Value> as LPARAM;
                        
                        unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
                            if IsWindowVisible(hwnd).as_bool() {
                                let mut title = [0u16; 512];
                                let len = GetWindowTextW(hwnd, &mut title);
                                if len > 0 {
                                    let title_str = String::from_utf16_lossy(&title[..len as usize]);
                                    
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
                                    
                                    let minimized = IsIconic(hwnd).as_bool();
                                    
                                    let windows_vec = &mut *(lparam.0 as *mut Vec<Value>);
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
                    },
                    "window.inspect" => {
                        let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing window_id"))?;
                        let ptr_val = wid.strip_prefix("hwnd:").and_then(|s| s.parse::<usize>().ok()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Invalid window_id format"))?;
                        let hwnd = HWND(ptr_val as _);
                        
                        let mut title = [0u16; 512];
                        let len = GetWindowTextW(hwnd, &mut title);
                        let title_str = if len > 0 { String::from_utf16_lossy(&title[..len as usize]) } else { String::new() };
                        
                        let mut class_name = [0u16; 512];
                        let class_len = GetClassNameW(hwnd, &mut class_name);
                        let class_str = if class_len > 0 { String::from_utf16_lossy(&class_name[..class_len as usize]) } else { String::new() };
                        
                        let mut pid = 0;
                        GetWindowThreadProcessId(hwnd, Some(&mut pid));
                        
                        let mut rect: windows::Win32::Foundation::RECT = std::mem::zeroed();
                        let _ = GetWindowRect(hwnd, &mut rect);
                        
                        let visible = IsWindowVisible(hwnd).as_bool();
                        let minimized = IsIconic(hwnd).as_bool();
                        
                        Ok(json!({
                            "window_id": wid,
                            "title": title_str,
                            "class_name": class_str,
                            "process_id": pid,
                            "bounds": {
                                "x": rect.left,
                                "y": rect.top,
                                "width": rect.right - rect.left,
                                "height": rect.bottom - rect.top
                            },
                            "visible": visible,
                            "minimized": minimized
                        }))
                    },
                    "window.focus" | "window.activate" => {
                        let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing window_id"))?;
                        let ptr_val = wid.strip_prefix("hwnd:").and_then(|s| s.parse::<usize>().ok()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Invalid window_id format"))?;
                        let hwnd = HWND(ptr_val as _);
                        let _ = SetForegroundWindow(hwnd);
                        Ok(json!({"success": true}))
                    },
                    "window.close" => {
                        let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing window_id"))?;
                        let ptr_val = wid.strip_prefix("hwnd:").and_then(|s| s.parse::<usize>().ok()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Invalid window_id format"))?;
                        let hwnd = HWND(ptr_val as _);
                        let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
                        Ok(json!({"success": true}))
                    },
                    "window.minimize" => {
                        let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing window_id"))?;
                        let ptr_val = wid.strip_prefix("hwnd:").and_then(|s| s.parse::<usize>().ok()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Invalid window_id format"))?;
                        let hwnd = HWND(ptr_val as _);
                        let _ = ShowWindow(hwnd, SW_MINIMIZE);
                        Ok(json!({"success": true}))
                    },
                    "window.maximize" => {
                        let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing window_id"))?;
                        let ptr_val = wid.strip_prefix("hwnd:").and_then(|s| s.parse::<usize>().ok()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Invalid window_id format"))?;
                        let hwnd = HWND(ptr_val as _);
                        let _ = ShowWindow(hwnd, SW_MAXIMIZE);
                        Ok(json!({"success": true}))
                    },
                    "window.move" => {
                        let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing window_id"))?;
                        let ptr_val = wid.strip_prefix("hwnd:").and_then(|s| s.parse::<usize>().ok()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Invalid window_id format"))?;
                        let hwnd = HWND(ptr_val as _);
                        let x = input.get("x").and_then(|v| v.as_i64()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing x"))? as i32;
                        let y = input.get("y").and_then(|v| v.as_i64()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing y"))? as i32;
                        let _ = SetWindowPos(hwnd, HWND(0), x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
                        Ok(json!({"success": true}))
                    },
                    "window.resize" => {
                        let wid = input.get("window_id").and_then(|v| v.as_str()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing window_id"))?;
                        let ptr_val = wid.strip_prefix("hwnd:").and_then(|s| s.parse::<usize>().ok()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Invalid window_id format"))?;
                        let hwnd = HWND(ptr_val as _);
                        let w = input.get("width").and_then(|v| v.as_i64()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing width"))? as i32;
                        let h = input.get("height").and_then(|v| v.as_i64()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing height"))? as i32;
                        let _ = SetWindowPos(hwnd, HWND(0), 0, 0, w, h, SWP_NOMOVE | SWP_NOZORDER);
                        Ok(json!({"success": true}))
                    },
                    _ => Err(err(CapabilityErrorCode::Unsupported, "Unsupported capability"))
                }
            }
        }).await.map_err(|_| err(CapabilityErrorCode::Internal, "Thread pool error"))??;

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
    fn provider_id(&self) -> &str { &self.provider_id }
    fn runtime_id(&self) -> &str { &self.runtime_id }

    fn definitions(&self) -> Vec<CapabilityDefinition> {
        let caps = vec!["keyboard.type", "keyboard.press", "keyboard.hotkey"];
        caps.into_iter().map(|cap| {
            let mut def = CapabilityDefinition::basic(
                cap,
                "Keyboard operation",
                vec![CapabilityRuntime::Windows],
                Idempotency::NonIdempotent,
            );
            def.metadata.security_level = SecurityLevel::Sensitive;
            def.metadata.required_permissions = vec![cap.to_string()];
            def
        }).collect()
    }

    async fn execute(&self, context: CapabilityProviderContext) -> Result<CapabilityProviderResult, CapabilityError> {
        let req = context.request;
        let input = req.input.clone();
        let cap = req.capability_id.clone();
        
        let output = spawn_blocking(move || -> Result<Value, CapabilityError> {
            unsafe {
                match cap.as_str() {
                    "keyboard.type" => {
                        let text = input.get("text").and_then(|v| v.as_str()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing text"))?;
                        let mut inputs = Vec::new();
                        for c in text.encode_utf16() {
                            let mut ip = INPUT {
                                type_: INPUT_KEYBOARD,
                                Anonymous: INPUT_0 {
                                    ki: KEYBDINPUT {
                                        wVk: VIRTUAL_KEY(0),
                                        wScan: c,
                                        dwFlags: KEYEVENTF_UNICODE,
                                        time: 0,
                                        dwExtraInfo: 0,
                                    }
                                }
                            };
                            inputs.push(ip);
                            ip.Anonymous.ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
                            inputs.push(ip);
                        }
                        if !inputs.is_empty() {
                            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                        }
                        Ok(json!({"success": true}))
                    },
                    "keyboard.press" => {
                        let key = input.get("key").and_then(|v| v.as_str()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing key"))?;
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
                                if num >= 1 && num <= 24 {
                                    VIRTUAL_KEY(VK_F1.0 + num - 1)
                                } else {
                                    return Err(err(CapabilityErrorCode::InvalidInput, "Invalid key"));
                                }
                            },
                            s if s.len() == 1 => {
                                let c = s.chars().next().unwrap();
                                let mut k = VkKeyScanW(c as u16);
                                VIRTUAL_KEY((k & 0xFF) as u16)
                            },
                            _ => return Err(err(CapabilityErrorCode::InvalidInput, "Invalid key"))
                        };
                        
                        let mut inputs = vec![
                            INPUT {
                                type_: INPUT_KEYBOARD,
                                Anonymous: INPUT_0 {
                                    ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0 }
                                }
                            },
                            INPUT {
                                type_: INPUT_KEYBOARD,
                                Anonymous: INPUT_0 {
                                    ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0 }
                                }
                            }
                        ];
                        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                        Ok(json!({"success": true}))
                    },
                    "keyboard.hotkey" => {
                        let keys = input.get("keys").and_then(|v| v.as_array()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing keys"))?;
                        let mut vks = Vec::new();
                        for key_val in keys {
                            let key = key_val.as_str().ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Invalid key element"))?;
                            let vk = match key.to_lowercase().as_str() {
                                "ctrl" => VK_CONTROL,
                                "alt" => VK_MENU,
                                "shift" => VK_SHIFT,
                                "win" => VK_LWIN,
                                s if s.len() == 1 => {
                                    let c = s.chars().next().unwrap();
                                    VIRTUAL_KEY((VkKeyScanW(c as u16) & 0xFF) as u16)
                                },
                                _ => return Err(err(CapabilityErrorCode::InvalidInput, "Invalid key"))
                            };
                            vks.push(vk);
                        }
                        
                        let mut inputs = Vec::new();
                        for &vk in &vks {
                            inputs.push(INPUT {
                                type_: INPUT_KEYBOARD,
                                Anonymous: INPUT_0 {
                                    ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0 }
                                }
                            });
                        }
                        for &vk in vks.iter().rev() {
                            inputs.push(INPUT {
                                type_: INPUT_KEYBOARD,
                                Anonymous: INPUT_0 {
                                    ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0 }
                                }
                            });
                        }
                        if !inputs.is_empty() {
                            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                        }
                        Ok(json!({"success": true}))
                    },
                    _ => Err(err(CapabilityErrorCode::Unsupported, "Unsupported capability"))
                }
            }
        }).await.map_err(|_| err(CapabilityErrorCode::Internal, "Thread pool error"))??;

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
    fn provider_id(&self) -> &str { &self.provider_id }
    fn runtime_id(&self) -> &str { &self.runtime_id }

    fn definitions(&self) -> Vec<CapabilityDefinition> {
        let caps = vec!["mouse.move", "mouse.click", "mouse.double_click", "mouse.right_click", "mouse.scroll"];
        caps.into_iter().map(|cap| {
            let mut def = CapabilityDefinition::basic(
                cap,
                "Mouse operation",
                vec![CapabilityRuntime::Windows],
                Idempotency::NonIdempotent,
            );
            def.metadata.security_level = SecurityLevel::Sensitive;
            def.metadata.required_permissions = vec![cap.to_string()];
            def
        }).collect()
    }

    async fn execute(&self, context: CapabilityProviderContext) -> Result<CapabilityProviderResult, CapabilityError> {
        let req = context.request;
        let input = req.input.clone();
        let cap = req.capability_id.clone();
        
        let output = spawn_blocking(move || -> Result<Value, CapabilityError> {
            unsafe {
                let screen_w = GetSystemMetrics(SM_CXSCREEN) as f64;
                let screen_h = GetSystemMetrics(SM_CYSCREEN) as f64;
                
                match cap.as_str() {
                    "mouse.move" | "mouse.click" | "mouse.double_click" | "mouse.right_click" => {
                        let x = input.get("x").and_then(|v| v.as_i64()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing x"))?;
                        let y = input.get("y").and_then(|v| v.as_i64()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing y"))?;
                        
                        let nx = ((x as f64 * 65535.0) / screen_w) as i32;
                        let ny = ((y as f64 * 65535.0) / screen_h) as i32;
                        
                        let mut inputs = vec![INPUT {
                            type_: INPUT_MOUSE,
                            Anonymous: INPUT_0 {
                                mi: MOUSEINPUT {
                                    dx: nx,
                                    dy: ny,
                                    mouseData: 0,
                                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                                    time: 0,
                                    dwExtraInfo: 0,
                                }
                            }
                        }];
                        
                        if cap == "mouse.click" || cap == "mouse.double_click" {
                            inputs.push(INPUT {
                                type_: INPUT_MOUSE,
                                Anonymous: INPUT_0 {
                                    mi: MOUSEINPUT { dx: nx, dy: ny, mouseData: 0, dwFlags: MOUSEEVENTF_LEFTDOWN | MOUSEEVENTF_ABSOLUTE, time: 0, dwExtraInfo: 0 }
                                }
                            });
                            inputs.push(INPUT {
                                type_: INPUT_MOUSE,
                                Anonymous: INPUT_0 {
                                    mi: MOUSEINPUT { dx: nx, dy: ny, mouseData: 0, dwFlags: MOUSEEVENTF_LEFTUP | MOUSEEVENTF_ABSOLUTE, time: 0, dwExtraInfo: 0 }
                                }
                            });
                        }
                        
                        if cap == "mouse.double_click" {
                            inputs.push(INPUT {
                                type_: INPUT_MOUSE,
                                Anonymous: INPUT_0 {
                                    mi: MOUSEINPUT { dx: nx, dy: ny, mouseData: 0, dwFlags: MOUSEEVENTF_LEFTDOWN | MOUSEEVENTF_ABSOLUTE, time: 0, dwExtraInfo: 0 }
                                }
                            });
                            inputs.push(INPUT {
                                type_: INPUT_MOUSE,
                                Anonymous: INPUT_0 {
                                    mi: MOUSEINPUT { dx: nx, dy: ny, mouseData: 0, dwFlags: MOUSEEVENTF_LEFTUP | MOUSEEVENTF_ABSOLUTE, time: 0, dwExtraInfo: 0 }
                                }
                            });
                        }
                        
                        if cap == "mouse.right_click" {
                            inputs.push(INPUT {
                                type_: INPUT_MOUSE,
                                Anonymous: INPUT_0 {
                                    mi: MOUSEINPUT { dx: nx, dy: ny, mouseData: 0, dwFlags: MOUSEEVENTF_RIGHTDOWN | MOUSEEVENTF_ABSOLUTE, time: 0, dwExtraInfo: 0 }
                                }
                            });
                            inputs.push(INPUT {
                                type_: INPUT_MOUSE,
                                Anonymous: INPUT_0 {
                                    mi: MOUSEINPUT { dx: nx, dy: ny, mouseData: 0, dwFlags: MOUSEEVENTF_RIGHTUP | MOUSEEVENTF_ABSOLUTE, time: 0, dwExtraInfo: 0 }
                                }
                            });
                        }
                        
                        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                        Ok(json!({"success": true}))
                    },
                    "mouse.scroll" => {
                        let delta = input.get("delta").and_then(|v| v.as_i64()).ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "Missing delta"))? as i32;
                        let input_event = INPUT {
                            type_: INPUT_MOUSE,
                            Anonymous: INPUT_0 {
                                mi: MOUSEINPUT {
                                    dx: 0,
                                    dy: 0,
                                    mouseData: (delta * 120) as u32,
                                    dwFlags: MOUSEEVENTF_WHEEL,
                                    time: 0,
                                    dwExtraInfo: 0,
                                }
                            }
                        };
                        SendInput(&[input_event], std::mem::size_of::<INPUT>() as i32);
                        Ok(json!({"success": true}))
                    },
                    _ => Err(err(CapabilityErrorCode::Unsupported, "Unsupported capability"))
                }
            }
        }).await.map_err(|_| err(CapabilityErrorCode::Internal, "Thread pool error"))??;

        Ok(CapabilityProviderResult {
            output,
            artifacts: vec![],
            side_effects: vec!["mouse.coordinate_input".to_string()],
            metadata: json!({"native": true, "api": "Win32", "host_os": "windows"}),
        })
    }
}
"""

with open(r"c:\Users\DaRkAngeL\Desktop\cognyxos\runtime\capability\src\windows_providers.rs", "w", encoding="utf-8") as f:
    f.write(code)
