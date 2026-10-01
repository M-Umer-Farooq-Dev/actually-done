use serde_json::{json, Value};
use std::{fs, process::Command};

fn receipt(answer: &str, extra: &[&str]) -> (i32, Value) {
    let dir = tempfile::tempdir().unwrap();
    let init = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(init.status.success());
    fs::write(dir.path().join("auth.py"), "# synthetic\n").unwrap();
    let transcript = dir.path().join("session.jsonl");
    let user = json!({"type":"user","message":{"role":"user","content":"Update auth.py."}});
    let assistant = json!({"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":answer}]}});
    fs::write(&transcript, format!("{user}\n{assistant}\n")).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_actually-done"))
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
fn missing_transcript_is_useful_error() {
    let dir = tempfile::tempdir().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_actually-done"))
        .arg(dir.path())
        .args(["--session", "absent.jsonl", "--no-test"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(!result.stderr.is_empty());
    assert!(result.stdout.is_empty());
}
