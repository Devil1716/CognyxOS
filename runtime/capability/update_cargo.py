import os

cargo_toml_path = r"c:\Users\DaRkAngeL\Desktop\cognyxos\runtime\capability\Cargo.toml"

with open(cargo_toml_path, "r") as f:
    content = f.read()

if "image =" not in content:
    content += '\nimage = { version = "0.25", default-features = false, features = ["png"] }\n'

if "[target.'cfg(target_os = \"windows\")'.dependencies]" not in content:
    content += """
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Accessibility",
    "Win32_Graphics_Gdi",
    "Win32_System_Threading",
    "Win32_Storage_FileSystem",
    "Win32_UI_Input_KeyboardAndMouse"
] }
"""

with open(cargo_toml_path, "w") as f:
    f.write(content)
