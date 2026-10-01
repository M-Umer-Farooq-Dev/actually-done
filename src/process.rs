//! Argument-list execution with bounded waits and process-tree cleanup.
use crate::{models::*, transcript::redact};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

pub static INTERRUPTED: AtomicBool = AtomicBool::new(false);
pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub cleanup_error: Option<String>,
}

fn capture(file: &mut File) -> Result<String> {
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut bytes = vec![];
    file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).replace("\r\n", "\n"))
}
fn reap(child: &mut Child, seconds: u64) -> bool {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if start.elapsed() < Duration::from_secs(seconds) => {
                thread::sleep(Duration::from_millis(5));
            }
            _ => return false,
        }
    }
}
fn exit_code(status: ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status
            .code()
            .or_else(|| status.signal().map(|s| -s))
            .unwrap_or(-1)
    }
    #[cfg(windows)]
    {
        status.code().unwrap_or(-1)
    }
}
fn stop(child: &mut Child) -> Option<String> {
    #[cfg(windows)]
    let failed = {
        let exe = std::env::var_os("SystemRoot")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "C:\\Windows".into())
            .join("System32/taskkill.exe");
        // taskkill is itself bounded, so a stalled cleanup cannot stall the CLI.
        let mut killer = Command::new(exe)
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match &mut killer {
            Ok(k) => {
                let start = Instant::now();
                loop {
                    match k.try_wait() {
                        Ok(Some(s)) => break !s.success(),
                        Ok(None) if start.elapsed() < Duration::from_secs(10) => {
                            thread::sleep(Duration::from_millis(5))
                        }
                        _ => {
                            let _ = k.kill();
                            reap(k, 5);
                            break true;
                        }
                    }
                }
            }
            Err(_) => true,
        }
    };
    #[cfg(unix)]
    let failed = {
        // The child was created in a new process group. A negative PID targets
        // only that group; kill has no Rust-owned memory or pointer arguments.
        let rc = unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
        rc != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
    };
    let _ = child.kill();
    if !reap(child, 5) {
        return Some("Test process cleanup timed out".into());
    }
    failed.then(|| "Test process-tree cleanup could not be confirmed".into())
}
pub fn execute(command: &mut Command, timeout: f64, combined: bool) -> Result<Output> {
    let mut stdout = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut stderr = tempfile::tempfile().map_err(|e| e.to_string())?;
    command.stdout(Stdio::from(stdout.try_clone().map_err(|e| e.to_string())?));
    command.stderr(Stdio::from(
        if combined {
            stdout.try_clone()
        } else {
            stderr.try_clone()
        }
        .map_err(|e| e.to_string())?,
    ));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x00000200 | 0x08000000);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("Cannot launch tests: {e}"))?;
    let start = Instant::now();
    let mut cleanup_error = None;
    let mut timed_out = false;
    let code = loop {
        if INTERRUPTED.load(Ordering::Relaxed) {
            stop(&mut child);
            return Err("interrupted".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => break exit_code(status),
            Ok(None) => {}
            Err(e) => {
                stop(&mut child);
                return Err(e.to_string());
            }
        }
        if start.elapsed().as_secs_f64() >= timeout {
            timed_out = true;
            cleanup_error = stop(&mut child);
            break child.try_wait().ok().flatten().map(exit_code).unwrap_or(-1);
        }
        thread::sleep(Duration::from_millis(2));
    };
    Ok(Output {
        code,
        stdout: capture(&mut stdout)?,
        stderr: capture(&mut stderr)?,
        timed_out,
        cleanup_error,
    })
}
pub fn argv(command: &str, windows: bool) -> Result<Vec<String>> {
    let args = if windows {
        // Match shlex(posix=False): preserve backslashes and quoted strings.
        let mut args = vec![];
        let mut part = String::new();
        let mut quote = None;
        for ch in command.chars() {
            if let Some(q) = quote {
                part.push(ch);
                if ch == q {
                    quote = None;
                    args.push(std::mem::take(&mut part));
                }
            } else if ch.is_whitespace() {
                if !part.is_empty() {
                    args.push(std::mem::take(&mut part));
                }
            } else {
                if part.is_empty() && matches!(ch, '\"' | '\'') {
                    quote = Some(ch);
                }
                part.push(ch);
            }
        }
        if quote.is_some() {
            return Err("Malformed test command quoting".into());
        }
        if !part.is_empty() {
            args.push(part);
        }
        args.into_iter()
            .map(|s| {
                if s.starts_with('"') && s.ends_with('"') {
                    s[1..s.len() - 1].to_string()
                } else {
                    s
                }
            })
            .collect()
    } else {
        shell_words::split(command).map_err(|_| "Malformed test command quoting".to_string())?
    };
    if args.is_empty() {
        return Err("Test command is empty".into());
    }
    if args
        .iter()
        .any(|a| ["&&", "||", "|", ";", ">", ">>", "<"].contains(&a.as_str()))
    {
        return Err("Compound shell commands are unsupported; use a test script".into());
    }
    Ok(args)
}
fn executable(name: &str, root: &Path) -> String {
    if Path::new(name).is_absolute() || root.join(name).is_file() {
        return crate::paths::display(&crate::paths::resolve(&root.join(name)));
    }
    for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        let exts = if cfg!(windows) {
            vec!["", ".exe", ".cmd", ".bat", ".com"]
        } else {
            vec![""]
        };
        for ext in exts {
            let p = dir.join(format!("{name}{ext}"));
            if p.is_file() {
                return crate::paths::display(&p);
            }
        }
    }
    name.into()
}
pub fn test_command(command: &str, root: &Path) -> Result<Command> {
    let mut args = argv(command, cfg!(windows))?;
    args[0] = executable(&args[0], root);
    #[cfg(windows)]
    if ["cmd", "bat"].contains(
        &Path::new(&args[0])
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase()
            .as_str(),
    ) {
        if args
            .iter()
            .any(|a| a.chars().any(|ch| "&|<>^%!\"\r\n".contains(ch)))
        {
            return Err("Shell metacharacters are unsupported in Windows batch arguments".into());
        }
        use std::os::windows::process::CommandExt;
        let shell = std::env::var_os("SystemRoot")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "C:\\Windows".into())
            .join("System32/cmd.exe");
        let mut c = Command::new(crate::paths::resolve(&shell));
        let quoted = args
            .iter()
            .map(|a| {
                if a.contains([' ', '\t']) {
                    format!("\"{a}\"")
                } else {
                    a.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        c.args(["/d", "/s", "/c"])
            .raw_arg(format!("\"{quoted}\""))
            .current_dir(root);
        return Ok(c);
    }
    let mut c = Command::new(&args[0]);
    c.args(&args[1..]).current_dir(root);
    Ok(c)
}
pub fn guess(root: &Path) -> Option<String> {
    if std::fs::read_to_string(root.join("package.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .is_some_and(|v| v["scripts"]["test"].is_string())
    {
        return Some("npm test".into());
    }
    for (files, cmd) in [
        (vec!["pyproject.toml", "pytest.ini"], "pytest -q"),
        (vec!["go.mod"], "go test ./..."),
        (vec!["Cargo.toml"], "cargo test"),
    ] {
        if files.iter().any(|f| root.join(f).exists()) {
            return Some(cmd.into());
        }
    }
    None
}
pub fn tests(root: &Path, cmd: Option<&str>, timeout: f64, waived: bool) -> Result<TestResult> {
    let mut t = TestResult {
        command: cmd.map(redact),
        ..TestResult::default()
    };
    if waived {
        t.status = "waived".into();
        return Ok(t);
    }
    let Some(cmd) = cmd else {
        return Ok(t);
    };
    match test_command(cmd, root).and_then(|mut c| execute(&mut c, timeout, true)) {
        Ok(o) => {
            t.status = if o.timed_out {
                "timeout"
            } else if o.code == 0 {
                "passed"
            } else {
                "failed"
            }
            .into();
            t.returncode = Some(o.code);
            t.output = redact(&o.stdout);
            t.error = o.cleanup_error;
        }
        Err(e) if e == "interrupted" => return Err(e),
        Err(e) => {
            t.status = "error".into();
            t.error = Some(e);
        }
    }
    Ok(t)
}
