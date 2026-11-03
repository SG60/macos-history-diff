use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use assert_cmd::Command;
use predicates::str::contains;

fn mhd() -> Command {
    Command::cargo_bin("mhd").unwrap()
}

/// `cargo test` builds examples into `target/<profile>/examples`, next to `deps/`.
fn gen_versions_bin() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("gen_versions")
}

/// Returns a temp dir (kept alive by the caller) and a file with three saved versions.
/// Requires running outside the sandbox; see AGENTS.md.
fn versioned_file() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let out = StdCommand::new(gen_versions_bin())
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "gen_versions failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let file = dir.path().join("doc.txt");
    (dir, file)
}

fn stdout_of(args: &[&str], file: &Path) -> String {
    let output = mhd().args(args).arg(file).output().unwrap();
    assert!(output.status.success(), "{args:?} failed");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn list_on_missing_file_fails() {
    mhd()
        .args(["list", "/definitely/not/here.txt"])
        .assert()
        .failure()
        .stderr(contains("file not found"));
}

#[test]
fn file_without_saved_versions_explains_why() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("plain.txt");
    std::fs::write(&file, "hello\n").unwrap();

    mhd()
        .arg("list")
        .arg(&file)
        .assert()
        .success()
        .stdout(contains("current"))
        .stdout(contains("No saved versions"));
}

#[test]
fn diff_and_log_need_at_least_two_versions() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("plain.txt");
    std::fs::write(&file, "hello\n").unwrap();

    for args in [vec!["diff"], vec!["log"]] {
        mhd()
            .args(&args)
            .arg(&file)
            .assert()
            .failure()
            .stderr(contains("No saved versions"));
    }
}

#[test]
fn list_shows_each_version_with_current_last() {
    let (_dir, file) = versioned_file();

    let stdout = stdout_of(&["list"], &file);

    let rows: Vec<&str> = stdout.lines().skip(1).collect();
    assert_eq!(rows.len(), 3, "{stdout}");
    assert!(rows[2].ends_with("current"), "{stdout}");
}

#[test]
fn diff_defaults_to_second_newest_against_current() {
    let (_dir, file) = versioned_file();

    mhd()
        .arg("diff")
        .arg(&file)
        .assert()
        .success()
        .stdout(contains("-version two"))
        .stdout(contains("+version three"));
}

#[test]
fn diff_between_explicit_versions() {
    let (_dir, file) = versioned_file();

    mhd()
        .arg("diff")
        .arg(&file)
        .args(["0", "current"])
        .assert()
        .success()
        .stdout(contains("-version one"))
        .stdout(contains("+version three"));
}

#[test]
fn diff_rejects_out_of_range_version() {
    let (_dir, file) = versioned_file();

    mhd()
        .arg("diff")
        .arg(&file)
        .args(["0", "9"])
        .assert()
        .failure()
        .stderr(contains("out of range"));
}

#[test]
fn log_is_newest_first_unless_reversed() {
    let (_dir, file) = versioned_file();

    let newest_first = stdout_of(&["log"], &file);
    let newest = newest_first.find("── 1 → current").expect(&newest_first);
    let oldest = newest_first.find("── 0 → 1").expect(&newest_first);
    assert!(newest < oldest, "{newest_first}");

    let oldest_first = stdout_of(&["log", "--reverse"], &file);
    let oldest = oldest_first.find("── 0 → 1").expect(&oldest_first);
    let newest = oldest_first.find("── 1 → current").expect(&oldest_first);
    assert!(oldest < newest, "{oldest_first}");
}

#[test]
fn ui_help_lists_optional_file() {
    mhd()
        .args(["ui", "--help"])
        .assert()
        .success()
        .stdout(contains("[FILE]"));
}
