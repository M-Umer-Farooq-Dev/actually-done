use std::path::{Component, Path, PathBuf};

pub fn display(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\UNC\")
        .map(|s| format!(r"\\{s}"))
        .unwrap_or_else(|| text.strip_prefix(r"\\?\").unwrap_or(&text).to_string())
}
pub fn home() -> PathBuf {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
pub fn expand(path: &str) -> PathBuf {
    if path == "~" {
        home()
    } else if let Some(p) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
        home().join(p)
    } else {
        PathBuf::from(path)
    }
}
pub fn resolve(path: &Path) -> PathBuf {
    if let Ok(p) = path.canonicalize() {
        return PathBuf::from(display(&p));
    }
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    if let (Some(parent), Some(name)) = (abs.parent(), abs.file_name()) {
        if parent.exists() {
            return resolve(parent).join(name);
        }
    }
    let mut result = PathBuf::new();
    for c in abs.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(c.as_os_str()),
        }
    }
    result
}
pub fn key(text: &str) -> String {
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text.into()
    }
}
pub fn inside(path: &Path, roots: &[PathBuf]) -> bool {
    let p = key(&display(&resolve(path)));
    roots.iter().any(|r| {
        let r = key(&display(&resolve(r)));
        p == r || p.starts_with(&(r + std::path::MAIN_SEPARATOR_STR))
    })
}
pub fn normalize(raw: &str, root: &Path) -> Option<String> {
    let mut value = raw.trim().trim_matches(['`', '"']).replace('\\', "/");
    let rt = display(root)
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string();
    let windows = rt.as_bytes().get(1) == Some(&b':')
        || value.as_bytes().get(1) == Some(&b':')
        || rt.starts_with("//");
    if value.starts_with('/') || value.as_bytes().get(1) == Some(&b':') {
        if windows && value.starts_with('/') && !value.starts_with("//") {
            value = format!("{}{}", rt.get(..2).unwrap_or(""), value);
        }
        let lhs = if windows {
            value.to_lowercase()
        } else {
            value.clone()
        };
        let rhs = if windows {
            rt.to_lowercase()
        } else {
            rt.clone()
        };
        if lhs == rhs {
            return None;
        }
        if !lhs.starts_with(&(rhs + "/")) {
            return None;
        }
        value = value[rt.len() + 1..].into();
    }
    let mut parts = vec![];
    for part in value.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}
