use crate::{models::Result, paths::home};
use serde_json::{json, Value};
use std::path::Path;

pub struct Config {
    pub provider: String,
    pub test: Option<String>,
    pub timeout: f64,
    pub guess: bool,
}
fn validate(v: &Value) -> Result<()> {
    if !matches!(v["provider"].as_str(), Some("auto" | "claude" | "codex")) {
        return Err("Invalid config: provider must be auto, claude, or codex".into());
    }
    if !v["test"].is_null() && !v["test"].as_str().is_some_and(|s| !s.trim().is_empty()) {
        return Err("Invalid config: test must be a nonempty command string".into());
    }
    if !v["test_timeout"]
        .as_f64()
        .is_some_and(|t| t.is_finite() && t > 0.)
    {
        return Err("Invalid config: test_timeout must be a positive finite number".into());
    }
    if !v["guess_test"].is_boolean() {
        return Err("Invalid config: guess_test must be a boolean".into());
    }
    Ok(())
}
pub fn load(root: &Path, overrides: Value) -> Result<Config> {
    let mut values = json!({"provider":"auto","test":null,"test_timeout":120.,"guess_test":false});
    for path in [
        home().join(".config/actually-done/config.toml"),
        root.join(".actually-done.toml"),
    ] {
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                return Err(format!(
                    "Invalid config at {}: unreadable file",
                    path.display()
                ))
            }
        };
        let data: toml::Value = toml::from_str(&raw)
            .map_err(|_| format!("Invalid config at {}: TOMLDecodeError", path.display()))?;
        for key in ["provider", "test", "test_timeout", "guess_test"] {
            if let Some(v) = data.get(key) {
                values[key] = serde_json::to_value(v)
                    .map_err(|_| "Invalid config: unsupported value".to_string())?;
            }
        }
        validate(&values)?;
    }
    for key in ["provider", "test", "test_timeout", "guess_test"] {
        if let Some(v) = overrides.get(key).filter(|v| !v.is_null()) {
            values[key] = v.clone();
        }
    }
    validate(&values)?;
    Ok(Config {
        provider: values["provider"].as_str().unwrap().into(),
        test: values["test"].as_str().map(str::to_string),
        timeout: values["test_timeout"].as_f64().unwrap(),
        guess: values["guess_test"].as_bool().unwrap(),
    })
}
