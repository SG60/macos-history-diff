# macos-history-diff (`mhd`)

Show how a text file evolved, using the versions macOS keeps for documents (`NSFileVersion`).
Diffs are produced by `git diff --no-index`.

```
mhd list <FILE>                  # list versions (0 = oldest, `current` = latest)
mhd diff <FILE> [A] [B]          # diff two versions (default: second newest vs current)
mhd log  <FILE> [--reverse]      # successive diffs through history (newest first by default)
mhd ui   [FILE]                  # open a window to explore the timeline (or Open/drop a file)
```

Version selectors `A` and `B` are an index from `mhd list` or the word `current`.

`mhd ui` shows the versions newest first on the left, and the diff of the selected version
against the previous one on the right. Keys: `↑`/`↓` (or `k`/`j`) select, `B` marks a base
version so the diff compares the selected version against it, `Esc` clears the base,
`⌘O` opens a file, `⌘R` reloads. Diffs are computed in-process, not with git or delta.

If `delta` is on your PATH and output goes to a terminal, diffs are piped through it.
Set `NO_COLOR` to disable colour.

## Limitation: which files have versions

Versions are only recorded for files saved through apps that use macOS autosave/versions
(e.g. TextEdit, Pages). Plain writes from editors or the command line do not create versions,
and copying a file starts a new history.

## Requirements

- macOS (the crate does not build elsewhere)
- `git` on PATH
- Optional: `delta`

## Install

```
cargo install --path .
```
