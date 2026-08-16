//! Isolated Windows GUI test mode. Inactive unless `COGNYX_GUI_TEST=1`.
//!
//! This is not a user-facing feature. Production application.open must not
//! attach personal documents. When the flag is set, providers and tests may
//! only target `C:\CognyxOSTestWorkspace\`.

use std::path::{Path, PathBuf};

pub const GUI_TEST_ENV: &str = "COGNYX_GUI_TEST";
pub const GUI_TEST_DOCUMENT_ENV: &str = "COGNYX_GUI_TEST_DOCUMENT";
pub const TEST_WORKSPACE: &str = r"C:\CognyxOSTestWorkspace";
pub const GOLDEN_FILENAME: &str = "CognyxOS-Golden-Test.txt";
pub const GOLDEN_TITLE_MARKER: &str = "cognyxos-golden-test";

pub fn enabled() -> bool {
    matches!(
        std::env::var(GUI_TEST_ENV)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    )
}

pub fn test_workspace() -> PathBuf {
    PathBuf::from(TEST_WORKSPACE)
}

pub fn golden_document_path() -> PathBuf {
    std::env::var(GUI_TEST_DOCUMENT_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| test_workspace().join(GOLDEN_FILENAME))
}

pub fn is_protected_title(title: &str) -> bool {
    let title = title.to_ascii_lowercase();
    let needles = [
        ".env",
        "credentials",
        "credential",
        "secrets",
        "secret",
        "password",
        "passwd",
        "token",
    ];
    needles.iter().any(|needle| title.contains(needle))
}

pub fn is_protected_path(path: &str) -> bool {
    let normalized = normalize_windows_path(path);
    if normalized.contains(".env")
        || normalized.contains("credential")
        || normalized.contains("secret")
        || normalized.contains("password")
        || normalized.contains("token")
    {
        return true;
    }
    let workspace = normalize_windows_path(TEST_WORKSPACE);
    if normalized == workspace || normalized.starts_with(&(workspace + "\\")) {
        return false;
    }
    let home = normalize_windows_path(&std::env::var("USERPROFILE").unwrap_or_default());
    if !home.is_empty() {
        for folder in ["\\documents\\", "\\desktop\\", "\\downloads\\"] {
            if normalized.starts_with(&format!("{home}{folder}")) {
                return true;
            }
        }
    }
    true
}

fn normalize_windows_path(path: &str) -> String {
    let mut normalized = path.replace('/', "\\").to_ascii_lowercase();
    if let Some(stripped) = normalized.strip_prefix(r"\\?\") {
        normalized = stripped.to_string();
    }
    while normalized.ends_with('\\') && normalized.len() > 3 {
        normalized.pop();
    }
    normalized
}

pub fn is_test_owned_title(title: &str) -> bool {
    let title = title.to_ascii_lowercase();
    title.contains(GOLDEN_TITLE_MARKER) && !is_protected_title(&title)
}

pub fn is_notepad_application(name: &str, executable: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let executable = executable.replace('/', "\\").to_ascii_lowercase();
    name.contains("notepad")
        || executable.ends_with("\\notepad.exe")
        || executable.ends_with("notepad.exe")
}

/// Classic Win32 Notepad. Windows 11 Store/WinUI Notepad is single-instance and
/// may reuse personal tabs; System32 notepad.exe is a separate process we can own.
pub fn isolated_notepad_executable() -> Option<PathBuf> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    let path = PathBuf::from(windir).join("System32").join("notepad.exe");
    path.is_file().then_some(path)
}

pub fn validate_test_document(path: &Path) -> Result<PathBuf, String> {
    let workspace = test_workspace();
    let _ = std::fs::create_dir_all(&workspace);
    let requested = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    };
    let canonical_workspace = workspace.canonicalize().unwrap_or(workspace.clone());
    let canonical = match requested.canonicalize() {
        Ok(existing) => existing,
        Err(_) => requested.clone(),
    };
    let workspace_prefix = normalize_windows_path(&canonical_workspace.to_string_lossy());
    let path_text = normalize_windows_path(&canonical.to_string_lossy());
    let raw_text = normalize_windows_path(&requested.to_string_lossy());
    if is_protected_path(&raw_text) || is_protected_path(&path_text) {
        return Err(format!(
            "TEST_TARGET_UNSAFE: document '{}' is protected or outside the test workspace",
            requested.display()
        ));
    }
    let in_workspace = path_text.starts_with(&workspace_prefix)
        || raw_text.starts_with(&normalize_windows_path(TEST_WORKSPACE));
    if !in_workspace {
        return Err(format!(
            "TEST_TARGET_UNSAFE: document '{}' is outside {}",
            requested.display(),
            TEST_WORKSPACE
        ));
    }
    let name = requested
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !name.starts_with("cognyxos-") || !name.ends_with(".txt") {
        return Err(format!(
            "TEST_TARGET_UNSAFE: filename '{}' is not a CognyxOS test document",
            requested.display()
        ));
    }
    Ok(requested)
}

pub fn ensure_golden_document() -> Result<PathBuf, String> {
    let path = validate_test_document(&golden_document_path())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&path, b"").map_err(|error| error.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_and_token_titles_are_protected() {
        assert!(is_protected_title("*.env - Notepad"));
        assert!(is_protected_title("secrets.txt - Notepad"));
        assert!(is_protected_title("api token notes"));
        assert!(!is_protected_title("CognyxOS-Golden-Test.txt - Notepad"));
    }

    #[test]
    fn only_test_workspace_paths_are_allowed() {
        assert!(is_protected_path(r"C:\Users\someone\Documents\notes.txt"));
        assert!(is_protected_path(r"C:\Users\someone\Desktop\file.txt"));
        assert!(is_protected_path(r"C:\CognyxOSTestWorkspace\.env"));
        assert!(!is_protected_path(
            r"C:\CognyxOSTestWorkspace\CognyxOS-Golden-Test.txt"
        ));
        assert!(!is_protected_path(
            r"\\?\C:\CognyxOSTestWorkspace\CognyxOS-Golden-Test.txt"
        ));
    }

    #[test]
    fn golden_title_is_the_only_owned_marker() {
        assert!(is_test_owned_title("*CognyxOS-Golden-Test.txt - Notepad"));
        assert!(!is_test_owned_title("Untitled - Notepad"));
        assert!(!is_test_owned_title("*.env - Notepad"));
    }

    #[test]
    fn validate_rejects_personal_and_wrong_names() {
        assert!(validate_test_document(Path::new(r"C:\Windows\notepad.exe")).is_err());
        assert!(validate_test_document(Path::new(r"C:\CognyxOSTestWorkspace\notes.txt")).is_err());
        assert!(validate_test_document(Path::new(
            r"C:\CognyxOSTestWorkspace\CognyxOS-Golden-Test.txt"
        ))
        .is_ok());
    }

    #[test]
    fn notepad_identity_matches_discovered_names() {
        assert!(is_notepad_application(
            "Notepad",
            r"C:\Windows\System32\notepad.exe"
        ));
        assert!(!is_notepad_application(
            "Calculator",
            r"C:\Windows\System32\calc.exe"
        ));
    }
}
