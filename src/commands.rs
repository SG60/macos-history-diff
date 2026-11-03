use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow, bail};

use crate::diff;
use crate::selector;
use crate::versions::{self, Version};

pub const NO_HISTORY: &str = "No saved versions. Versions are only recorded for files saved through apps that use macOS autosave/versions (e.g. TextEdit, Pages), not plain writes from editors or the command line.";

pub fn list(file: &Path) -> Result<()> {
    let versions = versions::list_versions(file)?;

    println!(
        "{:>3}  {:<19}  {:<24}  {:>8}",
        "IDX", "MODIFIED (local)", "SAVED BY", "SIZE"
    );
    for v in &versions {
        let when = format_date(v);
        let saved_by = v.saved_by.as_deref().unwrap_or("-");
        let size = fs::metadata(&v.url).map_or("-".to_string(), |m| m.len().to_string());
        let marker = if v.is_current { "  current" } else { "" };
        println!(
            "{:>3}  {when:<19}  {saved_by:<24}  {size:>8}{marker}",
            v.index
        );
    }

    if versions.len() < 2 {
        println!("\n{NO_HISTORY}");
    }
    Ok(())
}

pub fn diff(file: &Path, a: Option<&str>, b: Option<&str>) -> Result<()> {
    let versions = versions::list_versions(file)?;
    require_history(&versions)?;

    let len = versions.len();
    let a_index = match a {
        Some(s) => selector::parse(s, len)?,
        None => len - 2,
    };
    let b_index = match b {
        Some(s) => selector::parse(s, len)?,
        None => len - 1,
    };
    let (va, vb) = (&versions[a_index], &versions[b_index]);

    let dir = tempfile::tempdir()?;
    let name = file_name(file);
    let pa = snapshot_or_skip(&dir, &name, va)?;
    let pb = snapshot_or_skip(&dir, &name, vb)?;

    println!(
        "--- {} ({})  →  {} ({})",
        version_name(va),
        format_date(va),
        version_name(vb),
        format_date(vb)
    );
    diff::print_diff(dir.path(), &pa, &pb)
}

pub fn log(file: &Path, reverse: bool) -> Result<()> {
    let versions = versions::list_versions(file)?;
    require_history(&versions)?;

    let dir = tempfile::tempdir()?;
    let name = file_name(file);
    let snapshots: Vec<Option<String>> = versions
        .iter()
        .map(|v| diff::snapshot(dir.path(), &name, v))
        .collect::<Result<_>>()?;

    // Each pair is identified by its newer version's index.
    let mut newer_indices: Vec<usize> = (1..versions.len()).collect();
    if !reverse {
        newer_indices.reverse();
    }

    for newer in newer_indices {
        let older = newer - 1;
        let (va, vb) = (&versions[older], &versions[newer]);
        let (Some(pa), Some(pb)) = (&snapshots[older], &snapshots[newer]) else {
            println!(
                "── {} → {} · skipped (binary file) ──",
                version_name(va),
                version_name(vb)
            );
            continue;
        };

        let (added, removed) = diff::numstat(dir.path(), pa, pb)?;
        if added == 0 && removed == 0 {
            continue;
        }
        println!(
            "── {} → {} · {} · +{added} −{removed} ──",
            version_name(va),
            version_name(vb),
            format_date(vb)
        );
        diff::print_diff(dir.path(), pa, pb)?;
        println!();
    }
    Ok(())
}

fn require_history(versions: &[Version]) -> Result<()> {
    if versions.len() < 2 {
        bail!("{NO_HISTORY}");
    }
    Ok(())
}

fn snapshot_or_skip(dir: &tempfile::TempDir, name: &str, v: &Version) -> Result<String> {
    diff::snapshot(dir.path(), name, v)?.ok_or_else(|| anyhow!("binary file, skipped"))
}

pub fn version_name(v: &Version) -> String {
    if v.is_current {
        "current".to_string()
    } else {
        v.index.to_string()
    }
}

pub fn format_date(v: &Version) -> String {
    v.modified.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn file_name(file: &Path) -> String {
    file.file_name()
        .map_or_else(|| "file".to_string(), |n| n.to_string_lossy().into_owned())
}
