use actually_done::{extract::claimed_done, transcript::redact};
use actually_done::{
    extract::leftovers,
    models::{Event, GitInfo, TestResult},
    paths::normalize,
    process::argv,
    run::verdict,
    transcript::{parse, validate_size, MAX_BYTES},
};
use std::path::Path;

#[test]
fn completion_negation_and_obligations_win() {
    assert!(claimed_done("Done. Tests pass."));
    for text in [
        "Not done",
        "Done. We should rotate the key later",
        "Done?",
        "```\nDone\n```",
    ] {
        assert!(!claimed_done(text), "{text}");
    }
}

#[test]
fn credentials_are_redacted_before_retention() {
    assert_eq!(redact("token=abc sk-test-secret"), "token=*** ***");
}

#[test]
fn sizes_are_checked_without_allocating_large_files() {
    assert!(validate_size(MAX_BYTES).is_ok());
    assert!(validate_size(MAX_BYTES + 1).is_err());
}

#[test]
fn failed_tools_are_attempts_not_successful_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("failed.jsonl");
    std::fs::write(&path,r#"{"type":"user","message":{"content":"Update auth.py"}}
{"type":"assistant","message":{"content":[{"type":"tool_use","id":"w1","name":"Write","input":{"file_path":"auth.py"}}]}}
{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"w1","content":{"exit_code":1}}]}}
{"type":"assistant","message":{"content":"Could not update auth.py"}}"#).unwrap();
    let s = parse(&path, "claude").unwrap();
    let c = actually_done::extract::extract(&s, dir.path(), &[], 0);
    assert_eq!(c.attempted, vec!["auth.py"]);
    assert!(c.written.is_empty());
    assert!(c.files.is_empty());
}

#[test]
fn completed_checkbox_closes_but_changed_filename_does_not() {
    let events = vec![
        Event {
            role: "user".into(),
            text: "- [ ] rotate key\nWe should migrate later".into(),
            ..Event::default()
        },
        Event {
            role: "assistant".into(),
            text: "- [x] rotate key\nUpdated auth.py".into(),
            ..Event::default()
        },
    ];
    let findings = leftovers(&events);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].text, "We should migrate later");
}

#[test]
fn leftover_limit_is_applied_after_later_resolutions() {
    let mut text = (0..25)
        .map(|i| format!("- [ ] TODO {i}\n"))
        .collect::<String>();
    text += "- [x] TODO 0\n";
    let f = leftovers(&[Event {
        role: "assistant".into(),
        text,
        ..Event::default()
    }]);
    assert_eq!(f.len(), 20);
    assert_eq!(f[0].text, "- [ ] TODO 1");
}

#[test]
fn long_obligations_are_detected_before_excerpt_truncation() {
    let prefix = "説明".repeat(110);
    let findings = leftovers(&[Event {
        role: "assistant".into(),
        text: format!(
            "{prefix} We should rotate the key later. {}",
            "more context ".repeat(20)
        ),
        ..Event::default()
    }]);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].text.chars().count() <= 200);
    assert_eq!(findings[0].text, findings[0].evidence_line);
    assert!(findings[0].text.ends_with('…'));
    assert!(findings[0].text.contains("rotate the key later"));
}

#[test]
fn long_checkbox_obligations_use_full_text_for_resolution_and_deduplication() {
    let prefix = "Detailed obligation ".repeat(15);
    let events = [Event {
        role: "assistant".into(),
        text: format!(
            "- [ ] {prefix}first\n- [ ] {prefix}second\n- [ ] {prefix}second\n- [x] {prefix}first"
        ),
        ..Event::default()
    }];
    assert_eq!(leftovers(&events).len(), 1);
}

#[test]
fn windows_paths_and_traversal() {
    let root = Path::new("D:/synthetic/repo");
    assert_eq!(
        normalize(r"d:\synthetic\repo\auth.py", root),
        Some("auth.py".into())
    );
    assert_eq!(normalize("../outside.py", root), None);
    assert_eq!(normalize("D:/synthetic/other/auth.py", root), None);
}

#[test]
fn command_quoting_preserves_windows_paths() {
    assert_eq!(
        argv(r#""C:\Program Files\python.exe" "test file.py""#, true).unwrap(),
        vec![r"C:\Program Files\python.exe", "test file.py"]
    );
    assert!(argv("pytest && echo done", false).is_err());
    assert!(argv("\"unfinished", true).is_err());
}

#[test]
fn verdict_precedence_is_shared_by_renderers() {
    let mut git = GitInfo::default();
    let mut tests = TestResult::default();
    assert_eq!(verdict(&git, &tests, false, true).0, "UNKNOWN");
    tests.status = "waived".into();
    assert_eq!(verdict(&git, &tests, false, true).1, 0);
    assert_eq!(verdict(&git, &tests, true, true).0, "PARTIAL");
    tests.status = "failed".into();
    assert_eq!(verdict(&git, &tests, true, true).0, "NOT DONE");
    git.error = Some("failure".into());
    assert_eq!(verdict(&git, &tests, true, true).1, 2);
}
