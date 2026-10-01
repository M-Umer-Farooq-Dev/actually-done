use crate::{models::*, paths::normalize};
use regex::Regex;
use std::{
    collections::{BTreeSet, HashSet},
    path::Path,
    sync::LazyLock,
};

static DONE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bdone\b|all set|should be good|tests pass|\bpassing\b|\bimplemented\b|\bcompleted\b|\bfinished\b|\blgtm\b|ready to commit").unwrap()
});
static NEGATIVE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:not|never|isn't|aren't|wasn't|weren't|couldn't|cannot|can't|haven't)\s+(?:yet\s+)?(?:done|completed|finished|implemented|ready|passing|pass|finish)\b|tests?\s+(?:do(?:es)?\s+not|don't|doesn't|didn't|failed|fail)\b|\b(?:remaining work|unfinished|not all|could not finish)\b").unwrap()
});
static UNFINISHED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^\s*[-*]\s*\[ \]|\bTODO\b|\bFIXME\b|we should|you should|don't forget|follow[- ]up|next step|\blater\b|not in this change|out of scope|left as|\bremaining\b").unwrap()
});
static LEFTOVER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*[-*]\s*\[ \]|\bTODO\b|\bFIXME\b|we should|you should|don't forget|follow[- ]up|as a next step|\bseparately\b|\blater\b|not in this change|out of scope for now|left as|\b(?:rotate|revoke|backup|migrate|hotfix)\b").unwrap()
});
static CHECK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[-*]\s*\[[ xX]\]\s*").unwrap());
static FINISHED_CHECK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[-*]\s*\[[xX]\]").unwrap());
static FILE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:[A-Za-z]:)?[\\/]?(?:[\w.@+~-]+[\\/])*[\w@+~-]+\.[A-Za-z0-9_]{1,12}").unwrap()
});
static FAILED_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:could not|unable|failed|unchanged|not changed)\b").unwrap()
});
static PATCH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\*\*\* (?:(?:Add|Update|Delete) File|Move to): (.+)$").unwrap()
});
static TEST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:pytest|npm\s+(?:run\s+)?test|go\s+test|cargo\s+test|unittest)\b").unwrap()
});
static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-z0-9]+").unwrap());
static SPLIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\n|;|\s+and\s+|(?:^|\s)\d+[.)]\s+").unwrap());
static BULLET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*[-*]\s*(?:\[[ xX]\]\s*)?").unwrap());
static VERB: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:add|fix|update|implement|create|remove|delete|test|write|build|refactor|support|handle|change|ensure|rotate|revoke|migrate|document|improve|check|verify|make|run|resolve)\b").unwrap()
});

pub fn prose(text: &str) -> Vec<&str> {
    let mut fence = None;
    let mut lines = vec![];
    for line in text.lines().map(str::trim) {
        if line.starts_with("```") || line.starts_with("~~~") {
            let marker = &line[..3];
            if fence.is_none() {
                fence = Some(marker);
            } else if fence == Some(marker) {
                fence = None;
            }
        } else if fence.is_none() && !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}
pub fn claimed_done(text: &str) -> bool {
    let p = prose(text).join("\n");
    !p.is_empty()
        && !p.contains('?')
        && !NEGATIVE.is_match(&p)
        && !UNFINISHED.is_match(&p)
        && DONE.is_match(&p)
}
pub fn leftovers(events: &[Event]) -> Vec<Finding> {
    let mut results: Vec<(String, Finding)> = vec![];
    for e in events {
        if e.role != "user" && e.role != "assistant" {
            continue;
        }
        for line in prose(&e.text) {
            if line.chars().count() > 200 {
                continue;
            }
            let normalized = CHECK
                .replace(line, "")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            if FINISHED_CHECK.is_match(line) {
                results.retain(|(k, _)| *k != normalized);
                continue;
            }
            if line.starts_with('{') || line.starts_with("[\"") || !LEFTOVER.is_match(line) {
                continue;
            }
            if !results.iter().any(|(k, _)| *k == normalized) {
                results.push((normalized, Finding::new(line, &e.role)));
            }
        }
    }
    results.into_iter().take(20).map(|(_, v)| v).collect()
}
fn tool_paths(t: &Tool, root: &Path) -> Vec<String> {
    let mut paths = vec![];
    for key in ["file_path", "path", "filename", "target_file", "file"] {
        if let Some(s) = t.input[key].as_str().and_then(|v| normalize(v, root)) {
            if !paths.contains(&s) {
                paths.push(s);
            }
        }
    }
    if t.name.to_lowercase().contains("patch") {
        if let Some(raw) = t
            .input
            .get("raw")
            .or_else(|| t.input.get("patch"))
            .and_then(|v| v.as_str())
        {
            for c in PATCH.captures_iter(raw) {
                if let Some(s) = normalize(c[1].trim_end_matches('\r'), root) {
                    if !paths.contains(&s) {
                        paths.push(s);
                    }
                }
            }
        }
    }
    paths
}
fn words(text: &str) -> HashSet<String> {
    WORD.find_iter(&text.to_lowercase())
        .map(|m| m.as_str().to_string())
        .filter(|w| {
            w.len() > 3
                && !["please", "with", "that", "this", "then", "also", "should"]
                    .contains(&w.as_str())
        })
        .collect()
}
fn sorted(items: Vec<String>) -> Vec<String> {
    items
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
pub fn extract(s: &Session, root: &Path, changed: &[String], prompt_index: usize) -> Claims {
    let mut c = Claims::default();
    let tools: Vec<_> = s.events.iter().flat_map(|e| &e.tools).collect();
    for t in &tools {
        let name = t.name.to_lowercase();
        let name = name.split('.').next_back().unwrap_or("");
        let paths = tool_paths(t, root);
        if ["read", "read_file", "readfile"].contains(&name) {
            c.read.extend(paths);
        } else if [
            "write",
            "write_file",
            "writefile",
            "edit",
            "edit_file",
            "multiedit",
        ]
        .contains(&name)
            || name.contains("patch")
        {
            c.attempted.extend(paths.clone());
            if t.success == Some(true) {
                if name.starts_with("write") {
                    c.written.extend(paths);
                } else {
                    c.edited.extend(paths);
                }
            }
        }
        let cmd = t.input.get("cmd").or_else(|| t.input.get("command"));
        let cmd = cmd
            .map(|v| {
                if let Some(a) = v.as_array() {
                    a.iter()
                        .map(|v| {
                            v.as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| v.to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    v.as_str().unwrap_or("").into()
                }
            })
            .unwrap_or_default();
        if TEST.is_match(&cmd) {
            let status = if t.is_error {
                "reported failed"
            } else if t.success == Some(true) {
                "reported successful"
            } else {
                "not verified"
            };
            let activity = format!("{} ({status})", cmd.chars().take(200).collect::<String>());
            if !c.activity.contains(&activity) {
                c.activity.push(activity);
            }
        }
    }
    c.files.extend(c.written.clone());
    c.files.extend(c.edited.clone());
    for line in prose(&s.final_text) {
        if NEGATIVE.is_match(line) || FAILED_PATH.is_match(line) {
            continue;
        }
        for m in FILE.find_iter(line) {
            let before = line[..m.start()].chars().next_back();
            let after = line[m.end()..].chars().next();
            if before.is_some_and(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.')
                || after.is_some_and(|ch| ch.is_alphanumeric() || ch == '_')
            {
                continue;
            }
            if let Some(path) = normalize(m.as_str(), root) {
                c.files.push(path);
            }
        }
    }
    c.files = sorted(c.files);
    c.read = sorted(c.read);
    c.written = sorted(c.written);
    c.edited = sorted(c.edited);
    c.attempted = sorted(c.attempted);
    c.done = claimed_done(&s.final_text) && !s.pending_user;
    c.leftovers = leftovers(&s.events);
    let path_words = words(&changed.join(" "));
    let final_words = words(&format!(
        "{} {}",
        s.final_text,
        c.leftovers
            .iter()
            .map(|f| f.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    ));
    let tool_words = words(
        &tools
            .iter()
            .filter(|t| t.success == Some(true))
            .map(|t| format!("{} {}", t.name, tool_paths(t, root).join(" ")))
            .collect::<Vec<_>>()
            .join(" "),
    );
    for raw in SPLIT.split(&s.prompts[prompt_index]) {
        let clause = BULLET.replace(raw, "");
        let clause = clause.trim();
        if !(4..=140).contains(&clause.chars().count()) || !VERB.is_match(clause) {
            continue;
        }
        let w = words(clause);
        if w.is_empty()
            || !w.is_disjoint(&path_words)
            || w.intersection(&final_words).count() >= 2
            || !w.is_disjoint(&tool_words)
        {
            continue;
        }
        let f = Finding::new(clause, "user");
        if !c.scope.contains(&f) {
            c.scope.push(f);
        }
    }
    c
}
