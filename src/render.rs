use crate::models::Receipt;
use std::fmt::Write;

fn safe(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch.is_control() {
                ch.escape_default().to_string()
            } else {
                ch.to_string()
            }
        })
        .collect()
}
fn list(out: &mut String, heading: &str, values: &[String], markdown: bool) {
    let _ = writeln!(
        out,
        "{}{}{}",
        if markdown { "\n### " } else { "\n" },
        heading,
        if markdown { "\n" } else { ":" }
    );
    if values.is_empty() {
        let _ = writeln!(out, "  (none)");
    }
    for value in values {
        let _ = writeln!(out, "- {}", safe(value));
    }
}
pub fn human(r: &Receipt, markdown: bool) -> String {
    let mut out = format!(
        "{}actually-done — {} / {}\n\n{}Task{} {}\n",
        if markdown { "# " } else { "" },
        safe(&r.provider),
        safe(&r.session_id),
        if markdown { "**" } else { "" },
        if markdown { ":**" } else { ":" },
        safe(&r.prompt)
    );
    let _ = writeln!(
        out,
        "\n{}Verdict: {} (exit {}){}",
        if markdown { "**" } else { "" },
        r.verdict,
        r.exit_code,
        if markdown { "**" } else { "" }
    );
    for reason in &r.reasons {
        let _ = writeln!(out, "{}", safe(reason));
    }
    let _ = writeln!(
        out,
        "\n{}Git{} {} | branch {}",
        if markdown { "## " } else { "" },
        if markdown { "\n" } else { ":" },
        if r.git.disabled {
            "disabled"
        } else if r.git.error.is_some() {
            "unavailable"
        } else {
            "current changes"
        },
        safe(r.git.branch.as_deref().unwrap_or("unknown"))
    );
    if let Some(error) = &r.git.error {
        let _ = writeln!(out, "{}", safe(error));
    }
    list(&mut out, "Claimed files", &r.claimed_files, markdown);
    list(&mut out, "Changed files", &r.git.changed_files, markdown);
    list(
        &mut out,
        "Not seen in current changes (heuristic)",
        &r.claimed_without_current_change,
        markdown,
    );
    let _ = writeln!(
        out,
        "\n{}Tests{} {}",
        if markdown { "## " } else { "" },
        if markdown { "\n" } else { ":" },
        r.tests.status
    );
    if let Some(cmd) = &r.tests.command {
        let _ = writeln!(out, "Command: {}", safe(cmd));
    }
    if let Some(code) = r.tests.returncode {
        let _ = writeln!(out, "Return code: {code}");
    }
    if let Some(error) = &r.tests.error {
        let _ = writeln!(out, "{}", safe(error));
    }
    list(
        &mut out,
        "Agent-reported test activity",
        &r.tests.agent_activity,
        markdown,
    );
    if !r.tests.output.is_empty() {
        let lines: Vec<_> = r.tests.output.lines().collect();
        let _ = writeln!(out, "\nFinal 40 test output lines:\n");
        // Encode controls and Markdown delimiters so transcript output cannot
        // inject terminal escapes or close a Markdown code fence.
        for line in &lines[lines.len().saturating_sub(40)..] {
            let _ = writeln!(out, "    {}", safe(line));
        }
    }
    list(
        &mut out,
        "Scope dropped (heuristic)",
        &r.scope_dropped
            .iter()
            .map(|f| format!("{} [{}]", f.text, f.source))
            .collect::<Vec<_>>(),
        markdown,
    );
    list(
        &mut out,
        "Leftovers",
        &r.leftovers
            .iter()
            .map(|f| format!("{} [{}]", f.text, f.source))
            .collect::<Vec<_>>(),
        markdown,
    );
    list(&mut out, "Evidence limits", &r.uncertainty, markdown);
    if r.token_totals.values().any(|v| *v > 0) {
        let _ = writeln!(out, "\nTokens: {:?}", r.token_totals);
    }
    out
}
