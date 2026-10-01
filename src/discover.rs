use crate::{
    models::*,
    paths::{expand, home, inside, resolve},
    transcript::{parse, records},
};
use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

struct Candidate {
    path: PathBuf,
    provider: String,
    id: Option<String>,
    cwd: Option<String>,
    modified: SystemTime,
}
fn header(path: &Path, hint: Option<&str>) -> Result<Candidate> {
    let mut c = Candidate {
        path: path.into(),
        provider: String::new(),
        id: None,
        cwd: None,
        modified: std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map_err(|_| "Cannot inspect transcript".to_string())?,
    };
    for row in records(path)?.take(200).flatten() {
        if crate::process::INTERRUPTED.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("interrupted".into());
        }
        match row["type"].as_str() {
            Some(kind @ ("session_meta" | "response_item" | "turn_context" | "event_msg")) => {
                c.provider = "codex".into();
                let p = &row["payload"];
                if c.cwd.is_none() {
                    c.cwd = p["cwd"].as_str().map(str::to_string);
                }
                if kind == "session_meta" {
                    if let Some(id) = p
                        .get("id")
                        .or_else(|| p.get("session_id"))
                        .and_then(|v| v.as_str())
                    {
                        c.id = Some(id.into());
                    }
                }
            }
            Some("user" | "assistant" | "system") if row.get("message").is_some() => {
                c.provider = "claude".into();
                if c.cwd.is_none() {
                    c.cwd = row["cwd"].as_str().map(str::to_string);
                }
                if let Some(id) = row["sessionId"].as_str() {
                    c.id = Some(id.into());
                }
            }
            _ => {}
        }
        if !c.provider.is_empty() && c.cwd.is_some() && c.id.is_some() {
            break;
        }
    }
    if c.provider.is_empty() {
        c.provider = hint
            .ok_or_else(|| "Cannot identify transcript provider".to_string())?
            .into();
    }
    Ok(c)
}
fn same_repo(cwd: Option<&str>, root: &Path) -> bool {
    let Some(cwd) = cwd else {
        return false;
    };
    let mut p = resolve(&expand(cwd));
    let root = resolve(root);
    if !inside(&p, std::slice::from_ref(&root)) {
        return false;
    }
    while p != root {
        if p.join(".git").exists() || !p.pop() {
            return false;
        }
    }
    true
}
fn files(root: &Path, allowed: &[PathBuf], found: &mut Vec<PathBuf>) {
    if !inside(root, allowed) {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let Ok(ty) = entry.file_type() else {
            continue;
        };
        if ty.is_dir()
            && !ty.is_symlink()
            && !["subagents", "tool-results", "node_modules", ".git"]
                .contains(&entry.file_name().to_string_lossy().as_ref())
        {
            files(&p, allowed, found);
        } else if !ty.is_dir() && p.extension().is_some_and(|s| s == "jsonl") && inside(&p, allowed)
        {
            found.push(p);
        }
    }
}
pub fn discover(root: &Path, provider: &str, session: Option<&str>) -> Result<Session> {
    let claude_env = std::env::var("ACTUALLY_DONE_CLAUDE_ROOT").ok();
    let codex_env = std::env::var("ACTUALLY_DONE_CODEX_ROOT").ok();
    let claude = claude_env
        .as_deref()
        .map(expand)
        .unwrap_or_else(|| home().join(".claude/projects"));
    let codex = codex_env
        .as_deref()
        .map(expand)
        .unwrap_or_else(|| home().join(".codex/sessions"));
    let archive = codex
        .parent()
        .unwrap_or(Path::new("."))
        .join("archived_sessions");
    let mut allowed = vec![home(), root.to_path_buf()];
    if claude_env.is_some() {
        allowed.push(claude.clone());
    }
    if codex_env.is_some() {
        allowed.push(codex.clone());
        allowed.push(archive.clone());
    }
    let roots = [("claude", claude), ("codex", codex), ("codex", archive)];
    let mut candidates = vec![];
    let mut notes = vec![];
    if let Some(manual) = session.map(expand).filter(|p| p.exists()) {
        let lexical = if manual.is_absolute() {
            manual.clone()
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(&manual)
        };
        if lexical != resolve(&manual) && !inside(&manual, &allowed) {
            return Err(
                "Transcript symlink escapes home, repo, and configured session roots".into(),
            );
        }
        let c = header(&resolve(&manual), None)?;
        if provider != "auto" && c.provider != provider {
            return Err("Explicit transcript provider does not match --provider".into());
        }
        candidates.push(c);
    } else {
        for (source, dir) in roots {
            if provider != "auto" && source != provider {
                continue;
            }
            let mut paths = vec![];
            files(&dir, &allowed, &mut paths);
            for p in paths {
                let c = match header(&p, Some(source)) {
                    Ok(c) => c,
                    Err(e) if e == "interrupted" => return Err(e),
                    Err(_) => {
                        notes.push(
                            "Skipped an unreadable, oversized, or unrecognized transcript".into(),
                        );
                        continue;
                    }
                };
                if c.provider != source {
                    continue;
                }
                let matches = if let Some(id) = session {
                    c.id.as_deref() == Some(id)
                        || p.file_stem().is_some_and(|s| {
                            s == id || s.to_string_lossy().ends_with(&format!("-{id}"))
                        })
                        || p.file_name().is_some_and(|s| s == id)
                } else {
                    same_repo(c.cwd.as_deref(), root)
                };
                if matches {
                    candidates.push(c);
                }
            }
        }
    }
    let selected = candidates
        .into_iter()
        .max_by(|a, b| {
            a.modified
                .cmp(&b.modified)
                .then_with(|| a.path.cmp(&b.path))
        })
        .ok_or_else(|| {
            "No matching session transcript found; supply --session PATH or ID".to_string()
        })?;
    let mut parsed = parse(&selected.path, &selected.provider)?;
    if parsed.prompts.is_empty() {
        return Err("Selected transcript contains no usable user prompts".into());
    }
    let parent = selected.path.parent().unwrap_or(Path::new("."));
    if parent
        .join(selected.path.file_stem().unwrap_or_default())
        .join("subagents")
        .exists()
        || parent.join("subagents").exists()
    {
        notes.push("Subagent transcripts exist and are excluded from v1 analysis".into());
    }
    if parent.join("tool-results").exists() {
        notes.push("External tool-result files exist and are excluded from v1 analysis".into());
    }
    if session.is_some() && !same_repo(parsed.cwd.as_deref(), root) {
        notes.push("Explicit session cwd does not match the target repo or is unavailable".into());
    }
    for note in notes {
        if !parsed.notes.contains(&note) {
            parsed.notes.push(note);
        }
    }
    Ok(parsed)
}
