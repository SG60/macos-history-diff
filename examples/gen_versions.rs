//! Creates `<dir>/doc.txt` with three saved versions so `mhd` can be exercised against real version history.
//! Must run outside the agent sandbox: the version store is not visible from inside it.

use std::path::{Path, PathBuf};
use std::{fs, thread, time::Duration};

use objc2::rc::Retained;
use objc2_foundation::{NSFileVersion, NSFileVersionAddingOptions, NSString, NSURL};

const VERSIONS: [&str; 3] = [
    "version one\n",
    "version two\nsecond line\n",
    "version three\nsecond line\nthird line\n",
];

fn file_url(path: &Path) -> Retained<NSURL> {
    NSURL::fileURLWithPath(&NSString::from_str(path.to_str().expect("utf8 path")))
}

fn main() -> Result<(), String> {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: gen_versions <fresh-dir>")?,
    );
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dir = fs::canonicalize(&dir).map_err(|e| e.to_string())?;
    let doc = dir.join("doc.txt");
    let snapshot = dir.join("snapshot.txt");
    if doc.exists() {
        return Err(format!(
            "{} already exists; use a fresh directory",
            doc.display()
        ));
    }

    fs::write(&doc, VERSIONS[0]).map_err(|e| e.to_string())?;
    for (previous, next) in VERSIONS.iter().zip(&VERSIONS[1..]) {
        // Store the outgoing contents as a version, then overwrite, as a save does.
        fs::write(&snapshot, previous).map_err(|e| e.to_string())?;
        NSFileVersion::addVersionOfItemAtURL_withContentsOfURL_options_error(
            &file_url(&doc),
            &file_url(&snapshot),
            NSFileVersionAddingOptions(0),
        )
        .map_err(|e| format!("addVersion failed: {}", e.localizedDescription()))?;
        // Keep modification dates distinct so version ordering is deterministic.
        thread::sleep(Duration::from_secs(1));
        fs::write(&doc, next).map_err(|e| e.to_string())?;
    }
    Ok(())
}
