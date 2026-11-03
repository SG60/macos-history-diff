use std::fs;
use std::path::Path;

use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Equal,
    Insert,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: Kind,
    pub old_no: Option<usize>,
    pub new_no: Option<usize>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Line(Line),
    /// A run of unchanged lines hidden to keep the view short.
    Gap(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Diff {
    pub rows: Vec<Row>,
    pub added: usize,
    pub removed: usize,
}

/// Reads a file as UTF-8 text. `None` if the content is not valid UTF-8 (treated as binary).
pub fn read_text(path: &Path) -> Option<String> {
    String::from_utf8(fs::read(path).ok()?).ok()
}

/// Line diff between `old` and `new`, showing `context` unchanged lines around each change.
pub fn diff(old: &str, new: &str, context: usize) -> Diff {
    let text_diff = TextDiff::from_lines(old, new);
    let mut lines = Vec::new();
    let (mut added, mut removed) = (0, 0);

    for change in text_diff.iter_all_changes() {
        let kind = match change.tag() {
            ChangeTag::Equal => Kind::Equal,
            ChangeTag::Insert => {
                added += 1;
                Kind::Insert
            }
            ChangeTag::Delete => {
                removed += 1;
                Kind::Delete
            }
        };
        lines.push(Line {
            kind,
            old_no: change.old_index().map(|i| i + 1),
            new_no: change.new_index().map(|i| i + 1),
            text: change.value().trim_end_matches(['\n', '\r']).to_string(),
        });
    }

    let changed: Vec<bool> = lines.iter().map(|l| l.kind != Kind::Equal).collect();
    let visible: Vec<bool> = (0..lines.len())
        .map(|i| {
            let lo = i.saturating_sub(context);
            let hi = (i + context).min(lines.len().saturating_sub(1));
            (lo..=hi).any(|j| changed[j])
        })
        .collect();

    let mut rows = Vec::new();
    let mut hidden = 0;
    for (line, shown) in lines.into_iter().zip(visible) {
        if shown {
            if hidden > 0 {
                rows.push(Row::Gap(hidden));
                hidden = 0;
            }
            rows.push(Row::Line(line));
        } else {
            hidden += 1;
        }
    }
    if hidden > 0 {
        rows.push(Row::Gap(hidden));
    }

    Diff {
        rows,
        added,
        removed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(d: &Diff) -> (usize, usize) {
        (d.added, d.removed)
    }

    #[test]
    fn identical_text_has_no_changes() {
        let d = diff("a\nb\n", "a\nb\n", 3);
        assert_eq!(counts(&d), (0, 0));
        assert_eq!(d.rows, vec![Row::Gap(2)]);
    }

    #[test]
    fn counts_inserts_and_deletes() {
        let d = diff("a\nb\nc\n", "a\nX\nY\nc\n", 0);
        assert_eq!(counts(&d), (2, 1));
    }

    #[test]
    fn distant_unchanged_lines_are_collapsed() {
        let old: String = (0..20).map(|i| format!("line {i}\n")).collect();
        let new = old.replace("line 10\n", "changed\n");
        let d = diff(&old, &new, 1);
        assert!(d.rows.contains(&Row::Gap(8)), "{:?}", d.rows);
        assert!(matches!(d.rows.first(), Some(Row::Gap(9))));
    }

    #[test]
    fn line_numbers_track_both_sides() {
        let d = diff("a\nb\n", "a\nc\nb\n", 3);
        let inserted = d.rows.iter().find_map(|r| match r {
            Row::Line(l) if l.kind == Kind::Insert => Some(l.clone()),
            _ => None,
        });
        let inserted = inserted.expect("an inserted line");
        assert_eq!((inserted.old_no, inserted.new_no), (None, Some(2)));
        assert_eq!(inserted.text, "c");
    }

    #[test]
    fn read_text_rejects_non_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bin");
        fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        assert_eq!(read_text(&path), None);
    }
}
