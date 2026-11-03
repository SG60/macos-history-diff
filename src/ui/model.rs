use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::textdiff::{self, Diff};
use crate::versions::{self, Version};

/// Lines of unchanged context kept around each change.
const CONTEXT: usize = 3;

/// Timeline state for one file. Versions are ordered oldest (0) to newest (last).
pub struct Model {
    pub file: PathBuf,
    pub versions: Vec<Version>,
    texts: Vec<Option<String>>,
    /// `(added, removed)` relative to the previous version; `None` for the oldest or binary versions.
    stats: Vec<Option<(usize, usize)>>,
    pub selected: usize,
    /// Version marked with `b`; when set, the diff is `base → selected` instead of `previous → selected`.
    pub base: Option<usize>,
}

/// The two versions a diff compares. `old` is `None` when there is no earlier version to compare against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair {
    pub old: Option<usize>,
    pub new: usize,
}

impl Model {
    pub fn load(file: &Path) -> Result<Self> {
        let versions = versions::list_versions(file)?;
        let texts = versions
            .iter()
            .map(|v| textdiff::read_text(&v.url))
            .collect();
        Ok(Self::from_parts(file.to_path_buf(), versions, texts))
    }

    pub fn from_parts(file: PathBuf, versions: Vec<Version>, texts: Vec<Option<String>>) -> Self {
        let mut stats = vec![None; versions.len()];
        for i in 1..versions.len() {
            stats[i] = match (&texts[i - 1], &texts[i]) {
                (Some(old), Some(new)) => {
                    let d = textdiff::diff(old, new, 0);
                    Some((d.added, d.removed))
                }
                _ => None,
            };
        }
        let selected = versions.len().saturating_sub(1);
        Self {
            file,
            versions,
            texts,
            stats,
            selected,
            base: None,
        }
    }

    pub fn has_history(&self) -> bool {
        self.versions.len() >= 2
    }

    pub fn stats(&self, index: usize) -> Option<(usize, usize)> {
        self.stats.get(index).copied().flatten()
    }

    pub fn is_binary(&self, index: usize) -> bool {
        self.texts.get(index).is_some_and(Option::is_none)
    }

    pub fn select(&mut self, index: usize) {
        if !self.versions.is_empty() {
            self.selected = index.min(self.versions.len() - 1);
        }
    }

    pub fn select_next(&mut self) {
        self.select(self.selected + 1);
    }

    pub fn select_prev(&mut self) {
        self.select(self.selected.saturating_sub(1));
    }

    /// Marks the selected version as the base, or clears the base if it is already the selected one.
    pub fn toggle_base(&mut self) {
        self.base = if self.base == Some(self.selected) {
            None
        } else {
            Some(self.selected)
        };
    }

    pub fn clear_base(&mut self) {
        self.base = None;
    }

    /// The pair shown in the diff pane, or `None` when the selected version is its own base.
    pub fn pair(&self) -> Option<Pair> {
        if self.versions.is_empty() {
            return None;
        }
        let new = self.selected;
        match self.base {
            Some(base) if base == new => None,
            Some(base) => Some(Pair {
                old: Some(base.min(new)),
                new: base.max(new),
            }),
            None => Some(Pair {
                old: new.checked_sub(1),
                new,
            }),
        }
    }

    /// Diff for the current pair. `None` if either side is binary or there is nothing to compare.
    pub fn diff(&self) -> Option<Diff> {
        let pair = self.pair()?;
        let new = self.texts[pair.new].as_deref()?;
        let old = match pair.old {
            Some(i) => self.texts[i].as_deref()?,
            None => "",
        };
        Some(textdiff::diff(old, new, CONTEXT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    fn model(texts: &[&str]) -> Model {
        let versions = (0..texts.len())
            .map(|index| Version {
                index,
                modified: Local::now(),
                saved_by: None,
                url: PathBuf::from(format!("v{index}")),
                is_current: index + 1 == texts.len(),
            })
            .collect();
        let texts = texts.iter().map(|t| Some(t.to_string())).collect();
        Model::from_parts(PathBuf::from("doc.txt"), versions, texts)
    }

    #[test]
    fn selects_newest_by_default_and_compares_with_previous() {
        let m = model(&["a\n", "a\nb\n", "a\nb\nc\n"]);
        assert_eq!(m.selected, 2);
        assert_eq!(
            m.pair(),
            Some(Pair {
                old: Some(1),
                new: 2
            })
        );
        assert_eq!(m.stats(2), Some((1, 0)));
        assert_eq!(m.stats(0), None);
    }

    #[test]
    fn oldest_version_compares_against_nothing() {
        let mut m = model(&["a\n", "b\n"]);
        m.select(0);
        assert_eq!(m.pair(), Some(Pair { old: None, new: 0 }));
        assert_eq!(m.diff().unwrap().added, 1);
    }

    #[test]
    fn base_compares_selected_against_marked_version_in_either_order() {
        let mut m = model(&["a\n", "b\n", "c\n", "d\n"]);
        m.select(0);
        m.toggle_base();
        m.select(3);
        assert_eq!(
            m.pair(),
            Some(Pair {
                old: Some(0),
                new: 3
            })
        );
        m.select(0);
        assert_eq!(m.pair(), None);
        m.clear_base();
        assert!(m.pair().is_some());
    }

    #[test]
    fn toggle_base_on_base_clears_it() {
        let mut m = model(&["a\n", "b\n"]);
        m.toggle_base();
        assert_eq!(m.base, Some(1));
        m.toggle_base();
        assert_eq!(m.base, None);
    }

    #[test]
    fn selection_is_clamped() {
        let mut m = model(&["a\n", "b\n"]);
        m.select(99);
        assert_eq!(m.selected, 1);
        m.select_prev();
        m.select_prev();
        assert_eq!(m.selected, 0);
        m.select_next();
        m.select_next();
        m.select_next();
        assert_eq!(m.selected, 1);
    }

    #[test]
    fn binary_versions_have_no_diff_or_stats() {
        let versions = (0..2)
            .map(|index| Version {
                index,
                modified: Local::now(),
                saved_by: None,
                url: PathBuf::from(format!("v{index}")),
                is_current: index == 1,
            })
            .collect();
        let m = Model::from_parts(
            PathBuf::from("doc.txt"),
            versions,
            vec![Some("a\n".to_string()), None],
        );
        assert!(m.is_binary(1));
        assert_eq!(m.stats(1), None);
        assert!(m.diff().is_none());
    }
}
