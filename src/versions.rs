use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, Local};
use objc2::rc::Retained;
use objc2_foundation::{NSFileVersion, NSString, NSURL};

pub struct Version {
    pub index: usize,
    pub modified: DateTime<Local>,
    pub saved_by: Option<String>,
    pub url: PathBuf,
    pub is_current: bool,
}

pub fn list_versions(file: &Path) -> Result<Vec<Version>> {
    if !file.is_file() {
        bail!("file not found: {}", file.display());
    }
    let file = fs::canonicalize(file)?;
    let url = file_url(&file)?;
    let current = NSFileVersion::currentVersionOfItemAtURL(&url)
        .ok_or_else(|| anyhow!("no version information for {}", file.display()))?;

    let mut raw = vec![RawVersion::new(&current, true)];
    if let Some(others) = NSFileVersion::otherVersionsOfItemAtURL(&url) {
        for other in others.iter() {
            raw.push(RawVersion::new(&other, false));
        }
    }

    raw.sort_by(|a, b| a.epoch.total_cmp(&b.epoch));
    Ok(raw
        .into_iter()
        .enumerate()
        .map(|(index, v)| v.into_version(index))
        .collect())
}

struct RawVersion {
    epoch: f64,
    saved_by: Option<String>,
    url: PathBuf,
    is_current: bool,
}

impl RawVersion {
    fn new(version: &NSFileVersion, is_current: bool) -> Self {
        Self {
            epoch: version
                .modificationDate()
                .map_or(0.0, |d| d.timeIntervalSince1970()),
            saved_by: version
                .localizedNameOfSavingComputer()
                .map(|s| s.to_string()),
            url: url_path(&version.URL()),
            is_current,
        }
    }

    fn into_version(self, index: usize) -> Version {
        let modified = UNIX_EPOCH + Duration::from_secs_f64(self.epoch.max(0.0));
        Version {
            index,
            modified: DateTime::<Local>::from(modified),
            saved_by: self.saved_by,
            url: self.url,
            is_current: self.is_current,
        }
    }
}

fn file_url(path: &Path) -> Result<Retained<NSURL>> {
    let path = path
        .to_str()
        .ok_or_else(|| anyhow!("path is not valid UTF-8: {}", path.display()))?;
    Ok(NSURL::fileURLWithPath(&NSString::from_str(path)))
}

fn url_path(url: &NSURL) -> PathBuf {
    url.path()
        .map(|p| PathBuf::from(p.to_string()))
        .unwrap_or_default()
}
