use crate::{
    models::*,
    paths::{display, resolve},
    process::execute,
    transcript::redact,
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
};

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let mut c = Command::new("git");
    c.args(args)
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0");
    let o = execute(&mut c, 10., false).map_err(|e| {
        if e == "interrupted" {
            e
        } else {
            format!("Git unavailable: {e}")
        }
    })?;
    if o.timed_out {
        return Err("Git unavailable: TimeoutExpired".into());
    }
    if o.code != 0 {
        return Err(format!(
            "Git failed: {}",
            redact(o.stderr.trim())
                .chars()
                .take(500)
                .collect::<String>()
        ));
    }
    Ok(o.stdout)
}
pub fn find_root(path: &Path) -> Result<PathBuf> {
    git(path, &["rev-parse", "--show-toplevel"])
        .map(|s| resolve(Path::new(s.trim())))
        .map_err(|e| {
            if e == "interrupted" {
                e
            } else {
                format!("Target is not an accessible git repo: {e}")
            }
        })
}
fn split(s: String) -> Vec<String> {
    s.split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
pub fn collect(root: &Path) -> Result<GitInfo> {
    let mut info = GitInfo {
        root: display(root),
        ..GitInfo::default()
    };
    let work = (|| -> Result<()> {
        let status = git(root, &["status", "--porcelain=v1", "-z", "-uall"])?;
        info.staged = split(git(root, &["diff", "--cached", "--name-only", "-z"])?);
        info.unstaged = split(git(root, &["diff", "--name-only", "-z"])?);
        info.branch = Some(
            git(root, &["rev-parse", "--abbrev-ref", "HEAD"])
                .or_else(|e| {
                    if e == "interrupted" {
                        Err(e)
                    } else {
                        git(root, &["symbolic-ref", "--short", "HEAD"])
                    }
                })?
                .trim()
                .into(),
        );
        let mut changed: BTreeSet<String> =
            info.staged.iter().chain(&info.unstaged).cloned().collect();
        let mut entries = status.split('\0');
        while let Some(entry) = entries.next() {
            if entry.len() < 3 {
                continue;
            }
            let Some(name) = entry.get(3..) else {
                continue;
            };
            let code = &entry[..2];
            changed.insert(name.into());
            if code == "??" {
                info.untracked.push(name.into());
            } else if code.contains(['R', 'C']) {
                if let Some(original) = entries.next().filter(|s| !s.is_empty()) {
                    changed.insert(original.into());
                }
            }
        }
        info.changed_files = changed.into_iter().collect();
        info.untracked.sort();
        Ok(())
    })();
    if let Err(e) = work {
        if e == "interrupted" {
            return Err(e);
        }
        info = GitInfo {
            root: display(root),
            error: Some(e),
            ..GitInfo::default()
        };
    }
    Ok(info)
}
