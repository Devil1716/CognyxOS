import os

path = r"c:\Users\DaRkAngeL\Desktop\cognyxos\runtime\capability\src\windows_providers.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

content = content.replace("HWND(0)", "HWND(std::ptr::null_mut())")
content = content.replace("type_:", "r#type:")
content = content.replace("&mut windows as *mut Vec<Value> as LPARAM", "LPARAM(&mut windows as *mut Vec<Value> as isize)")
content = content.replace("lparam.0 as *mut Vec<Value>", "lparam.0 as *mut std::ffi::c_void as *mut Vec<Value>")
content = content.replace("LPARAM(0)", "LPARAM(0 as isize)")
content = content.replace("WPARAM(0)", "WPARAM(0 as usize)")

with open(path, "w", encoding="utf-8") as f:
    f.write(content)
