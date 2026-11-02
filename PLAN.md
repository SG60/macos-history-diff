# macos-history-diff — MVP implementation plan

Goal: a macOS-only Rust CLI that shows how a text file evolved, using the macOS file versions
infrastructure (`NSFileVersion`). Diffing is delegated to `git diff --no-index`.

Rules for the implementer:
- Work through the steps in order. Run `cargo build` and `cargo clippy` after each step.
- Do not invent API signatures. If a method name is uncertain, check the objc2-foundation source
  (`~/.cargo/registry/src/*/objc2-foundation-*/src/generated/NSFileVersion.rs`) and follow compiler errors.
- Keep all `unsafe`/ObjC code in `src/versions.rs`. No other module may import `objc2*`.
- Comments only where non-obvious. No extra features beyond this plan.

## Real-world fixture

The repo contains `example textedit doc.txt` (note the spaces), a file saved by TextEdit that has several
real versions. Use it as the primary manual and integration test target. Do not modify, move or
re-save it (that would change its version history). If tests need to write, use temp copies/files instead.
Quote the path in all shell commands.

## 0. Commands (final CLI surface)

```
mhd list <FILE>                  # list versions
mhd diff <FILE> [A] [B]          # diff two versions
mhd log  <FILE> [--reverse]      # successive diffs through history
```
Version selector (`A`, `B`): integer index from `list` (0 = oldest) or `current`.
Defaults for `diff`: A = second newest, B = `current` (the latest change).
Binary name `mhd` (`[[bin]] name = "mhd"`; package name `macos-history-diff`).

## 1. Scaffold

- `cargo init --bin` in repo root; add `.gitignore` with `/target`.
- Dependencies:
  - `objc2` and `objc2-foundation` (`default-features = false`; features `std`, `NSFileVersion`, `NSURL`,
    `NSDate`, `NSArray`, `NSString`, `NSError`, `NSObject`, plus any the compiler requires).
  - `clap` (`derive`), `anyhow`, `chrono` (`clock`, `std`), `tempfile`.
  - dev-deps: `assert_cmd`, `predicates`.
- `main.rs`: `compile_error!` on non-macOS.
- Modules: `main.rs` (clap + dispatch), `versions.rs` (ObjC), `selector.rs`, `diff.rs`, `commands.rs`.

## 2. Spike (do BEFORE the rest; report and stop if it fails)

Throwaway `examples/spike.rs` (delete afterwards) that:
1. Takes a path, canonicalizes it, builds an `NSURL` (`fileURLWithPath`).
2. Calls `NSFileVersion::currentVersionOfItemAtURL` and `otherVersionsOfItemAtURL`.
3. Prints for each: URL path, `modificationDate`, `localizedNameOfSavingComputer`.
4. Run it on `example textedit doc.txt`. Success = multiple versions listed with readable, distinct contents.
5. Also try `addVersionOfItemAtURL:withContentsOfURL:options:error:` on a temp file to see whether synthetic versions
   can be created for tests (needed for step 7 if so).

If `NSFileVersion` is not exposed, or no versions are returned for the example file (permissions/TCC, e.g. the terminal may
need Full Disk Access, or the file's location matters), STOP and report. Fallback would be a Swift helper; do not start it without asking.

## 3. `versions.rs`

```rust
pub struct Version {
    pub index: usize,              // 0 = oldest; current has the highest index
    pub modified: chrono::DateTime<chrono::Local>,
    pub saved_by: Option<String>,  // localizedNameOfSavingComputer
    pub url: PathBuf,              // stored contents location
    pub is_current: bool,
}
pub fn list_versions(file: &Path) -> anyhow::Result<Vec<Version>>;
```
- `NSDate` -> `timeIntervalSince1970()`.
- Collect other versions plus the current one, sort ascending by modified date (stable), assign indices, flag current.
- Missing file -> `file not found` error. No other versions -> return just the current one.
- Convert `NSError` into `anyhow` via `localizedDescription`.
- Only if the spike shows it works: `pub fn add_snapshot(file, contents)` for tests.

## 4. `selector.rs`

`parse(s, len) -> Result<usize>`: `current` -> last; integer bounds-checked; otherwise an error stating the valid range.
Unit tests: `current`, `0`, out of range, garbage, empty list.

## 5. `diff.rs`

- Copy a version's content to a `tempfile::TempDir` as `<basename>@<index>_<YYYYmmdd-HHMMSS>` (stored versions live in the
  protected `.DocumentRevisions-V100` dir; copy for stability).
- Reject non-UTF-8 (`std::str::from_utf8`): return a "binary file, skipped" error (in `log`, print a one-line note and continue).
- Run `git --no-pager diff --no-index --color=auto -- <a> <b>`. Exit 0 = identical, 1 = differences (both OK); others are errors.
- If `delta` is on PATH and stdout is a TTY, pipe `--color=always` output into it; otherwise print directly. Respect `NO_COLOR`.
- Clear error if `git` is missing.

## 6. `commands.rs`

- `list`: table `IDX  MODIFIED (local)  SAVED BY  SIZE`, marking `current`. If only the current version exists, print:
  "No saved versions. Versions are only recorded for files saved through apps that use macOS autosave/versions
  (e.g. TextEdit, Pages), not plain writes from editors or the command line."
- `diff`: resolve selectors; error if fewer than 2 versions; print a header with version labels, then the diff.
- `log`: for each consecutive pair print `── <prev> → <idx> · <date> · +N −M ──` (counts from `git diff --no-index --numstat`),
  then the diff. Newest first by default, `--reverse` for oldest first. Skip identical pairs.

## 7. Tests

- Integration tests in `tests/` with `assert_cmd` against `example textedit doc.txt` (path via `env!("CARGO_MANIFEST_DIR")`):
  `list` shows >= 2 rows; `diff 0 current` exits 0 and prints a diff; `log` prints at least one header;
  missing file exits non-zero. Prefer asserting structure rather than exact content/timestamps.
- If the environment cannot read versions, mark tests `#[ignore]` with a comment explaining why.
- Gate: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`.

## 8. Docs

Short `README.md`: purpose, usage above, the versions limitation, `cargo install --path .`.

## Definition of done

- `cargo build --release`, `cargo test`, `cargo clippy -- -D warnings` pass.
- `mhd list/diff/log "example textedit doc.txt"` show sensible output.
- Final report: spike outcome, anything skipped, API quirks found.
