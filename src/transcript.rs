use crate::models::*;
use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    sync::LazyLock,
};

pub const MAX_BYTES: u64 = 50 * 1024 * 1024;
static KEY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bsk-[A-Za-z0-9_-]+").unwrap());
static CREDENTIAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(\b(?:api[_-]?key|key|token|password|secret)\s*[=:]\s*)(?:"[^"]*"|'[^']*'|[^\s,;}]+)"#).unwrap()
});
static CODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:exit(?:ed)?(?:\s+with)?(?:\s+code)?|returncode)\s*[:=]?\s*(-?\d+)").unwrap()
});
static FAILED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^\s*(?:error:|failed:|apply_patch verification failed)").unwrap()
});

pub fn redact(text: &str) -> String {
    CREDENTIAL
        .replace_all(&KEY.replace_all(text, "***"), "${1}***")
        .into_owned()
}
fn redact_value(v: Value) -> Value {
    match v {
        Value::String(s) => Value::String(redact(&s)),
        Value::Array(a) => Value::Array(a.into_iter().map(redact_value).collect()),
        Value::Object(m) => {
            Value::Object(m.into_iter().map(|(k, v)| (k, redact_value(v))).collect())
        }
        v => v,
    }
}
pub fn text_content(v: &Value) -> String {
    if let Some(s) = v.as_str() {
        return redact(s);
    }
    v.as_array()
        .map(|a| {
            a.iter()
                .filter(|b| {
                    matches!(
                        b["type"].as_str(),
                        Some("text" | "input_text" | "output_text")
                    )
                })
                .map(|b| text_content(&b["text"]))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}
fn timestamp(v: &Value) -> Option<String> {
    let s = v.as_str()?;
    let date = DateTime::parse_from_rfc3339(s).ok().or_else(|| {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
            .ok()
            .map(|d| d.and_utc().fixed_offset())
    })?;
    let fraction = if date.timestamp_subsec_micros() == 0 {
        String::new()
    } else {
        format!(".{:06}", date.timestamp_subsec_micros())
    };
    Some(format!(
        "{}{}{}",
        date.format("%Y-%m-%d %H:%M:%S"),
        fraction,
        date.format("%:z")
    ))
}
pub fn validate_size(size: u64) -> Result<()> {
    if size > MAX_BYTES {
        Err("Transcript exceeds the 50 MB limit".into())
    } else {
        Ok(())
    }
}
pub fn records(path: &Path) -> Result<impl Iterator<Item = Option<Value>>> {
    let f = File::open(path).map_err(|_| "Cannot read transcript".to_string())?;
    validate_size(
        f.metadata()
            .map_err(|_| "Cannot inspect transcript".to_string())?
            .len(),
    )?;
    Ok(BufReader::new(f)
        .split(b'\n')
        .filter_map(|line| match line {
            Ok(bytes) => {
                let s = String::from_utf8_lossy(&bytes);
                if s.trim().is_empty() {
                    return None;
                }
                Some(
                    serde_json::from_str::<Value>(&s)
                        .ok()
                        .filter(Value::is_object),
                )
            }
            Err(_) => Some(None),
        }))
}
fn counts(v: &Value) -> Usage {
    [
        ("input", "input_tokens", "input_tokens"),
        ("output", "output_tokens", "output_tokens"),
        (
            "cache_read",
            "cache_read_input_tokens",
            "cached_input_tokens",
        ),
        (
            "cache_write",
            "cache_creation_input_tokens",
            "cache_creation_input_tokens",
        ),
    ]
    .into_iter()
    .map(|(k, a, b)| {
        (
            k.into(),
            v.get(a)
                .or_else(|| v.get(b))
                .and_then(Value::as_u64)
                .unwrap_or(0),
        )
    })
    .collect()
}
fn maximum(dest: &mut Usage, src: Usage) {
    for (k, v) in src {
        let entry = dest.entry(k).or_default();
        *entry = (*entry).max(v);
    }
}
fn sum(usages: &HashMap<String, Usage>) -> Usage {
    let mut result = counts(&Value::Null);
    for u in usages.values() {
        for (k, v) in u {
            *result.entry(k.clone()).or_default() += v;
        }
    }
    result
}
fn complete(tool: &mut Tool, out: &Value, error: bool) {
    let raw = out
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| out.to_string());
    let parsed = serde_json::from_str::<Value>(&raw).ok();
    let obj = if out.is_object() {
        out
    } else {
        parsed.as_ref().unwrap_or(&Value::Null)
    };
    let code = obj
        .get("exit_code")
        .or_else(|| obj.get("returncode"))
        .and_then(Value::as_i64)
        .or_else(|| CODE.captures(&raw).and_then(|c| c[1].parse().ok()));
    let failed = error
        || obj["is_error"] == true
        || obj["isError"] == true
        || code.is_some_and(|c| c != 0)
        || FAILED.is_match(&raw);
    tool.success = Some(!failed);
    tool.is_error = failed;
}
fn item(
    item: &Value,
    date: &Value,
    model: &Option<String>,
    events: &mut Vec<Event>,
    calls: &mut HashMap<String, Tool>,
    seen: &mut HashSet<String>,
) {
    match item["type"].as_str() {
        Some("message") => {
            let role = match item["role"].as_str() {
                Some("developer") => "system",
                Some(s @ ("user" | "assistant" | "system")) => s,
                _ => return,
            };
            if let Some(id) = item["id"].as_str() {
                if !seen.insert(format!("message:{id}")) {
                    return;
                }
            }
            let text = text_content(&item["content"]);
            if !text.is_empty() {
                events.push(Event {
                    ts: timestamp(date),
                    role: role.into(),
                    text,
                    model: model.clone(),
                    final_response: role == "assistant" && item["phase"] != "commentary",
                    ..Event::default()
                });
            }
        }
        Some("function_call" | "custom_tool_call") => {
            let id = item
                .get("call_id")
                .or_else(|| item.get("id"))
                .map(value_string)
                .unwrap_or_else(|| format!("anonymous-{}", calls.len()));
            if calls.contains_key(&id) {
                return;
            }
            let mut input = item
                .get("arguments")
                .or_else(|| item.get("input"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            if let Some(s) = input.as_str() {
                input = serde_json::from_str(s).unwrap_or_else(|_| json!({"raw":s}));
            }
            if !input.is_object() {
                input = json!({"raw":value_string(&input)});
            }
            let tool = Tool {
                id: id.clone(),
                name: item["name"].as_str().unwrap_or("unknown").into(),
                input: redact_value(input),
                ..Tool::default()
            };
            calls.insert(id, tool.clone());
            events.push(Event {
                ts: timestamp(date),
                role: "assistant".into(),
                tools: vec![tool],
                model: model.clone(),
                ..Event::default()
            });
        }
        Some("function_call_output" | "custom_tool_call_output") => {
            if let Some(id) = item["call_id"].as_str() {
                if let Some(tool) = calls.get_mut(id) {
                    if !seen.insert(format!("output:{id}")) {
                        return;
                    }
                    complete(
                        tool,
                        item.get("output").unwrap_or(&json!("")),
                        item["is_error"] == true,
                    );
                    events.push(Event {
                        ts: timestamp(date),
                        role: "tool".into(),
                        ..Event::default()
                    });
                }
            }
        }
        _ => {}
    }
}
fn value_string(v: &Value) -> String {
    v.as_str()
        .map(str::to_string)
        .unwrap_or_else(|| v.to_string())
}

pub fn parse(path: &Path, provider: &str) -> Result<Session> {
    let mut s = Session {
        provider: provider.into(),
        path: crate::paths::display(path),
        ..Session::default()
    };
    let mut calls = HashMap::<String, Tool>::new();
    let mut seen = HashSet::<String>::new();
    let mut usages = HashMap::<String, Usage>::new();
    let mut cumulative: Option<Usage> = None;
    let mut model = None;
    for (index, row) in records(path)?.enumerate() {
        if crate::process::INTERRUPTED.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("interrupted".into());
        }
        let Some(row) = row else {
            s.warnings += 1;
            continue;
        };
        let kind = row["type"].as_str().unwrap_or("");
        if provider == "claude" {
            if row["isSidechain"] == true {
                continue;
            }
            if let Some(id) = row["uuid"].as_str() {
                if !seen.insert(id.into()) {
                    continue;
                }
            }
            if let Some(id) = row["sessionId"].as_str() {
                s.id = id.into();
            }
            if let Some(cwd) = row["cwd"].as_str() {
                s.cwd = Some(cwd.into());
            }
            if !matches!(kind, "user" | "assistant" | "system") {
                continue;
            }
            let msg = &row["message"];
            let mut tools = vec![];
            let mut has_result = false;
            if let Some(blocks) = msg["content"].as_array() {
                for b in blocks {
                    if b["type"] == "tool_use" {
                        let id = b
                            .get("id")
                            .map(value_string)
                            .unwrap_or_else(|| format!("anonymous-{index}-{}", tools.len()));
                        if b.get("id").is_some_and(|v| !v.is_string()) {
                            continue;
                        }
                        if let std::collections::hash_map::Entry::Vacant(entry) =
                            calls.entry(id.clone())
                        {
                            let input = if b["input"].is_object() {
                                redact_value(b["input"].clone())
                            } else {
                                json!({})
                            };
                            let t = Tool {
                                id: id.clone(),
                                name: b["name"].as_str().unwrap_or("unknown").into(),
                                input,
                                ..Tool::default()
                            };
                            entry.insert(t.clone());
                            tools.push(t);
                        }
                    } else if b["type"] == "tool_result" {
                        has_result = true;
                        if let Some(t) = b["tool_use_id"].as_str().and_then(|id| calls.get_mut(id))
                        {
                            let mut output = b.get("content").cloned().unwrap_or_else(|| json!(""));
                            if output.is_array() {
                                output = json!(text_content(&output));
                            }
                            complete(t, &output, b["is_error"] == true);
                        }
                    }
                }
            }
            if kind == "assistant" && msg["usage"].is_object() {
                let id = msg
                    .get("id")
                    .map(value_string)
                    .unwrap_or_else(|| format!("record-{index}"));
                maximum(usages.entry(id).or_default(), counts(&msg["usage"]));
            }
            let text = text_content(&msg["content"]);
            let ts = timestamp(&row["timestamp"]);
            if has_result {
                s.events.push(Event {
                    ts: ts.clone(),
                    role: "tool".into(),
                    ..Event::default()
                });
            }
            if !text.is_empty() || !tools.is_empty() {
                s.events.push(Event {
                    ts,
                    role: kind.into(),
                    final_response: kind == "assistant" && !text.is_empty() && tools.is_empty(),
                    text,
                    tools,
                    model: msg["model"].as_str().map(str::to_string),
                });
            }
        } else {
            let p = &row["payload"];
            if kind == "session_meta" {
                if let Some(id) = p
                    .get("id")
                    .or_else(|| p.get("session_id"))
                    .and_then(Value::as_str)
                {
                    s.id = id.into();
                }
            }
            match kind {
                "session_meta" | "turn_context" => {
                    if let Some(cwd) = p["cwd"].as_str() {
                        s.cwd = Some(cwd.into());
                    }
                    if let Some(m) = p["model"].as_str() {
                        model = Some(m.into());
                    }
                }
                "response_item" => item(
                    p,
                    &row["timestamp"],
                    &model,
                    &mut s.events,
                    &mut calls,
                    &mut seen,
                ),
                "event_msg" => match p["type"].as_str() {
                    Some("item_completed") => item(
                        &p["item"],
                        &row["timestamp"],
                        &model,
                        &mut s.events,
                        &mut calls,
                        &mut seen,
                    ),
                    Some(kind @ ("user_message" | "agent_message" | "task_complete")) => {
                        let text = text_content(
                            p.get("last_agent_message")
                                .or_else(|| p.get("message"))
                                .unwrap_or(&Value::Null),
                        );
                        let role = if kind == "user_message" {
                            "user"
                        } else {
                            "assistant"
                        };
                        if !text.is_empty()
                            && !s
                                .events
                                .iter()
                                .rev()
                                .find(|e| !e.text.is_empty())
                                .is_some_and(|e| e.role == role && e.text == text)
                        {
                            s.events.push(Event {
                                ts: timestamp(&row["timestamp"]),
                                role: role.into(),
                                text,
                                model: model.clone(),
                                final_response: role == "assistant",
                                ..Event::default()
                            });
                        }
                    }
                    Some("token_count") if p["info"]["total_token_usage"].is_object() => maximum(
                        cumulative.get_or_insert_with(Usage::new),
                        counts(&p["info"]["total_token_usage"]),
                    ),
                    _ => {}
                },
                "token_usage_record" => {
                    if p["thread_token_usage"].is_object() {
                        maximum(
                            cumulative.get_or_insert_with(Usage::new),
                            counts(&p["thread_token_usage"]),
                        );
                    } else if p["usage"].is_object() {
                        usages.insert(
                            p.get("response_id")
                                .map(value_string)
                                .unwrap_or_else(|| format!("usage-{index}")),
                            counts(&p["usage"]),
                        );
                    }
                }
                _ => {}
            }
        }
    }
    for e in &mut s.events {
        for tool in &mut e.tools {
            if let Some(t) = calls.get(&tool.id) {
                *tool = t.clone();
            }
        }
    }
    let last_user = s
        .events
        .iter()
        .rposition(|e| e.role == "user" && !e.text.is_empty());
    let assistants: Vec<_> = s
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.role == "assistant" && !e.text.is_empty())
        .collect();
    let current: Vec<_> = assistants
        .iter()
        .filter(|(i, _)| last_user.is_none_or(|u| *i > u))
        .collect();
    s.pending_user = current.is_empty();
    s.final_text = current
        .iter()
        .rev()
        .find(|(_, e)| e.final_response)
        .or_else(|| current.last())
        .map(|(_, e)| e.text.clone())
        .or_else(|| assistants.last().map(|(_, e)| e.text.clone()))
        .unwrap_or_default();
    s.prompts = s
        .events
        .iter()
        .filter(|e| e.role == "user" && !e.text.is_empty())
        .map(|e| e.text.clone())
        .collect();
    let dates: Vec<_> = s.events.iter().filter_map(|e| e.ts.as_ref()).collect();
    // Compare instants even when providers preserve distinct UTC offsets.
    let instant = |date: &&String| {
        DateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S%.f%:z")
            .map(|d| d.with_timezone(&Utc))
            .ok()
    };
    s.started = dates
        .iter()
        .min_by_key(|d| instant(d))
        .map(|d| (*d).clone());
    s.ended = dates
        .iter()
        .max_by_key(|d| instant(d))
        .map(|d| (*d).clone());
    for e in &s.events {
        if let Some(m) = &e.model {
            if !s.models.contains(m) {
                s.models.push(m.clone());
            }
        }
    }
    s.usage = cumulative.unwrap_or_else(|| sum(&usages));
    if s.id.is_empty() {
        s.id = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
    }
    Ok(s)
}
