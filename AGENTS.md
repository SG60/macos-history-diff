# Agent notes for macos-history-diff

## Sandbox and macOS file versions

Inside the agent sandbox, `NSFileVersion` cannot see the system version store
(`/System/Volumes/Data/.DocumentRevisions-V100`). `otherVersionsOfItemAtURL` returns an empty
list and `addVersionOfItemAtURL` fails with `NSCocoaErrorDomain` code 512, even when the same
commands work outside the sandbox.

Commands that create, read, or list file versions (`cargo run --example ...`, `cargo test`,
`mhd list/diff/log`) may therefore fail or show misleading results inside the sandbox. When one
of these is needed:

- Ask the user to run the command manually outside the sandbox and paste the output, or
- Request a sandbox bypass for that individual command, if the environment offers one.

Do not treat an empty version list from inside the sandbox as evidence that a file has no history.

# Version control

You can use `jj` for version control.
