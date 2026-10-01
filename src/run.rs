use crate::{
    config, discover, extract, gitinfo,
    models::*,
    paths::{display, key, resolve},
    process,
};
use serde_json::json;
use std::path::PathBuf;

#[derive(Default)]
pub struct Options {
    pub path: PathBuf,
    pub session: Option<String>,
    pub provider: Option<String>,
    pub test: Option<String>,
    pub timeout: Option<f64>,
    pub guess: bool,
    pub no_git: bool,
    pub no_test: bool,
    pub prompt_index: usize,
    pub out: Option<PathBuf>,
}
pub fn validate_output(out: &std::path::Path, transcript: &std::path::Path) -> Result<()> {
    if resolve(out) == resolve(transcript)
        || (out.exists()
            && same_file::is_same_file(out, transcript)
                .map_err(|_| "Cannot validate output path".to_string())?)
    {
        Err("--out must not overwrite the selected transcript".into())
    } else {
        Ok(())
    }
}
pub fn verdict(
    git: &GitInfo,
    tests: &TestResult,
    concerns: bool,
    done: bool,
) -> (String, i32, Vec<String>) {
    let (v, c, r) = if let Some(e) = &git.error {
        ("UNKNOWN", 2, e.clone())
    } else if tests.status == "failed" {
        (
            "NOT DONE",
            1,
            "Fresh tests returned a nonzero exit code".into(),
        )
    } else if concerns {
        (
            "PARTIAL",
            1,
            "Heuristic file, scope, or leftover concerns remain".into(),
        )
    } else if git.disabled {
        ("UNKNOWN", 1, "Git verification was disabled".into())
    } else if !["passed", "waived"].contains(&tests.status.as_str()) {
        (
            "UNKNOWN",
            1,
            tests
                .error
                .clone()
                .unwrap_or_else(|| format!("Tests are {}", tests.status)),
        )
    } else if !done {
        (
            "UNKNOWN",
            1,
            "No current completion claim was detected".into(),
        )
    } else {
        ("DONE", 0, "no gaps detected by the available checks".into())
    };
    (v.into(), c, vec![r])
}
pub fn run(o: Options) -> Result<Receipt> {
    let target = resolve(&o.path);
    if !target.is_dir() {
        return Err("Target path must be an existing directory".into());
    }
    let root = if o.no_git {
        target
    } else {
        gitinfo::find_root(&target)?
    };
    let cfg = config::load(
        &root,
        json!({"provider":o.provider,"test":o.test,"test_timeout":o.timeout,"guess_test":if o.guess { Some(true) } else { None }}),
    )?;
    let s = discover::discover(&root, &cfg.provider, o.session.as_deref())?;
    if o.prompt_index >= s.prompts.len() {
        return Err(format!(
            "Prompt index must be between 0 and {}",
            s.prompts.len() - 1
        ));
    }
    if let Some(out) = &o.out {
        validate_output(out, std::path::Path::new(&s.path))?;
    }
    let mut git = if o.no_git {
        GitInfo {
            root: display(&root),
            disabled: true,
            ..GitInfo::default()
        }
    } else {
        gitinfo::collect(&root)?
    };
    let command = cfg.test.or_else(|| {
        if cfg.guess && !o.no_test {
            process::guess(&root)
        } else {
            None
        }
    });
    let mut tests = if git.error.is_some() {
        TestResult {
            command: command.as_deref().map(crate::transcript::redact),
            ..TestResult::default()
        }
    } else {
        process::tests(&root, command.as_deref(), cfg.timeout, o.no_test)?
    };
    if !git.disabled && git.error.is_none() && tests.returncode.is_some() {
        git = gitinfo::collect(&root)?;
    }
    if let Some(out) = &o.out {
        validate_output(out, std::path::Path::new(&s.path))?;
    }
    let c = extract::extract(&s, &root, &git.changed_files, o.prompt_index);
    tests.agent_activity = c.activity;
    let unmatched: Vec<_> = if git.disabled || git.error.is_some() {
        vec![]
    } else {
        c.files
            .iter()
            .filter(|p| !git.changed_files.iter().any(|ch| key(ch) == key(p)))
            .cloned()
            .collect()
    };
    let extra = git
        .changed_files
        .iter()
        .filter(|p| !c.files.iter().any(|ch| key(ch) == key(p)))
        .cloned()
        .collect();
    let (verdict, exit_code, reasons) = verdict(
        &git,
        &tests,
        !unmatched.is_empty() || !c.leftovers.is_empty() || !c.scope.is_empty(),
        c.done,
    );
    let mut uncertainty = vec![
        "Current git changes cannot establish session authorship or detect already committed work."
            .into(),
        "Completion, leftovers, and dropped scope are heuristics, not proof of correctness.".into(),
        "Claims and leftovers span the selected transcript; it may contain multiple tasks.".into(),
    ];
    uncertainty.extend(s.notes);
    if s.warnings > 0 {
        uncertainty.push(format!(
            "Skipped {} malformed transcript records.",
            s.warnings
        ));
    }
    if tests.status == "waived" {
        uncertainty.push("Tests were explicitly waived with --no-test.".into());
    }
    if tests.status == "unverified" {
        uncertainty.push("No fresh test result is available.".into());
    }
    if tests.returncode.is_some() && !git.disabled {
        uncertainty.push(
            "Git was inspected after fresh tests; their file effects may be included.".into(),
        );
    }
    Ok(Receipt {
        schema_version: "1".into(),
        provider: s.provider,
        session_id: s.id,
        session_path: s.path,
        started_at: s.started,
        ended_at: s.ended,
        models: s.models,
        event_count: s.events.len(),
        prompt: s.prompts[o.prompt_index].clone(),
        prompt_index: o.prompt_index,
        claimed_done: c.done,
        claimed_files: c.files,
        claimed_without_current_change: unmatched,
        changed_but_unclaimed: extra,
        files_read: c.read,
        files_written: c.written,
        files_edited: c.edited,
        attempted_files: c.attempted,
        git,
        tests,
        leftovers: c.leftovers,
        scope_dropped: c.scope,
        verdict,
        exit_code,
        reasons,
        uncertainty,
        parse_warnings: s.warnings,
        token_totals: s.usage,
    })
}
