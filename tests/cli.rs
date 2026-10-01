use serde_json::{json, Value};
use std::{fs, process::Command};

fn isolated_cli(home: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_actually-done"));
    command
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ACTUALLY_DONE_CLAUDE_ROOT", home.join(".claude/projects"))
        .env("ACTUALLY_DONE_CODEX_ROOT", home.join(".codex/sessions"));
    command
}

fn receipt(answer: &str, extra: &[&str]) -> (i32, Value) {
    receipt_messages(&[answer], extra)
}

fn receipt_messages(answers: &[&str], extra: &[&str]) -> (i32, Value) {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let init = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(init.status.success());
    fs::write(dir.path().join("auth.py"), "# synthetic\n").unwrap();
    let transcript = dir.path().join("session.jsonl");
    let user = json!({"type":"user","message":{"role":"user","content":"Update auth.py."}});
    let mut records = format!("{user}\n");
    for answer in answers {
        let assistant = json!({"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":answer}]}});
        records.push_str(&format!("{assistant}\n"));
    }
    fs::write(&transcript, records).unwrap();
    let result = isolated_cli(home.path())
        .arg(dir.path())
        .args(["--provider", "claude", "--session"])
        .arg(transcript)
        .arg("--json")
        .args(extra)
        .output()
        .unwrap();
    assert!(
        result.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    (
        result.status.code().unwrap(),
        serde_json::from_slice(&result.stdout).unwrap(),
    )
}

#[test]
fn matching_claim_with_explicit_waiver_is_done() {
    let (exit, data) = receipt("Done. Updated `auth.py`.", &["--no-test"]);
    assert_eq!(exit, 0);
    assert_eq!(data["verdict"], "DONE");
    assert_eq!(data["schema_version"], "1");
}

#[test]
fn unseen_claim_is_partial() {
    let (exit, data) = receipt("Done. Updated `missing.py`.", &["--no-test"]);
    assert_eq!(exit, 1);
    assert_eq!(data["verdict"], "PARTIAL");
}

#[test]
fn obligation_survives_completion() {
    let (exit, data) = receipt(
        "Done. Updated `auth.py`. We should rotate the key later.",
        &["--no-test"],
    );
    assert_eq!(exit, 1);
    assert_eq!(data["verdict"], "PARTIAL");
    assert!(!data["leftovers"].as_array().unwrap().is_empty());
}

#[test]
fn unconfigured_tests_are_unknown() {
    let (exit, data) = receipt("Done. Updated `auth.py`.", &[]);
    assert_eq!(exit, 1);
    assert_eq!(data["verdict"], "UNKNOWN");
}

#[test]
fn long_intermediate_obligation_prevents_done() {
    let obligation = format!(
        "We should rotate the key later. {}",
        "Context describing the outstanding obligation. ".repeat(6)
    );
    let (exit, data) = receipt_messages(&[&obligation, "Done. Updated `auth.py`."], &["--no-test"]);
    assert_eq!(exit, 1);
    assert_eq!(data["verdict"], "PARTIAL");
    assert_eq!(data["claimed_done"], true);
    let findings = data["leftovers"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert!(findings[0]["text"]
        .as_str()
        .unwrap()
        .contains("rotate the key later"));
}

#[test]
fn acceptance_tests_ignore_invoking_global_configuration() {
    for config in [
        "test = \"git --version\"",
        "test = \"git actually-done-intentionally-invalid-command\"",
        "test_timeout = -1",
    ] {
        let home = tempfile::tempdir().unwrap();
        let config_dir = home.path().join(".config/actually-done");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("config.toml"), config).unwrap();
        let result = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "unconfigured_tests_are_unknown", "--nocapture"])
            .env("HOME", home.path())
            .env("USERPROFILE", home.path())
            .env(
                "ACTUALLY_DONE_CLAUDE_ROOT",
                home.path().join("external-claude"),
            )
            .env(
                "ACTUALLY_DONE_CODEX_ROOT",
                home.path().join("external-codex/sessions"),
            )
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "Invoking config {config}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn missing_transcript_is_useful_error() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let result = isolated_cli(home.path())
        .arg(dir.path())
        .args(["--session", "absent.jsonl", "--no-test"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(!result.stderr.is_empty());
    assert!(result.stdout.is_empty());
}
