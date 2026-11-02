use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{Context, Result, anyhow, bail};

use crate::versions::Version;

/// Copies a version's contents into `dir`, returning the file name. `Ok(None)` for non-UTF-8 content.
pub fn snapshot(dir: &Path, file_name: &str, v: &Version) -> Result<Option<String>> {
    let bytes = fs::read(&v.url)
        .with_context(|| format!("reading version {} from {}", v.index, v.url.display()))?;
    if std::str::from_utf8(&bytes).is_err() {
        return Ok(None);
    }
    let name = format!(
        "{file_name}@{}_{}",
        v.index,
        v.modified.format("%Y%m%d-%H%M%S")
    );
    fs::write(dir.join(&name), bytes)?;
    Ok(Some(name))
}

pub fn print_diff(dir: &Path, a: &str, b: &str) -> Result<()> {
    let use_delta = io::stdout().is_terminal() && delta_available();
    let color = if std::env::var_os("NO_COLOR").is_some() {
        "--color=never"
    } else if use_delta {
        "--color=always"
    } else {
        "--color=auto"
    };

    if !use_delta {
        let status = git_diff(dir, color, a, b).status().map_err(git_error)?;
        return check_git_status(status);
    }

    let mut git = git_diff(dir, color, a, b)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(git_error)?;
    let pipe = git.stdout.take().context("git stdout was not captured")?;
    let mut delta = Command::new("delta")
        .stdin(pipe)
        .spawn()
        .context("starting delta")?;
    let delta_status = delta.wait()?;
    check_git_status(git.wait()?)?;
    if !delta_status.success() {
        bail!("delta exited with {delta_status}");
    }
    Ok(())
}

/// Returns `(lines added, lines removed)`.
pub fn numstat(dir: &Path, a: &str, b: &str) -> Result<(usize, usize)> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(["--no-pager", "diff", "--no-index", "--numstat", "--", a, b])
        .output()
        .map_err(git_error)?;
    check_git_status(output.status)?;

    let (mut added, mut removed) = (0, 0);
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut fields = line.split('\t');
        if let (Some(add), Some(del)) = (fields.next(), fields.next()) {
            added += add.parse::<usize>().unwrap_or(0);
            removed += del.parse::<usize>().unwrap_or(0);
        }
    }
    Ok((added, removed))
}

fn git_diff(dir: &Path, color: &str, a: &str, b: &str) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir)
        .args(["--no-pager", "diff", "--no-index", color, "--", a, b]);
    cmd
}

fn delta_available() -> bool {
    Command::new("delta")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

// `git diff --no-index` exits 1 when the files differ, which is not a failure.
fn check_git_status(status: ExitStatus) -> Result<()> {
    match status.code() {
        Some(0 | 1) => Ok(()),
        _ => bail!("git diff failed: {status}"),
    }
}

fn git_error(e: io::Error) -> anyhow::Error {
    if e.kind() == io::ErrorKind::NotFound {
        anyhow!("git not found on PATH; install git (e.g. Xcode Command Line Tools)")
    } else {
        anyhow!(e)
    }
}
