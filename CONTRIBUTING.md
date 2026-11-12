# Contributing

Ideas and bug reports are welcome as [issues](https://github.com/Edgar-Mendonca/Datum-Flow/issues/new). File › Roadmap in the app (and `docs/site.json`) shows what is planned.

## How the code is laid out

- `crates/flow-core`: the model (geometry, fluid, boundaries, settings), units, the `.dflow` file, undo and redo.
- `crates/flow-mesh`: the 2D mesher and its quality report.
- `crates/flow-solve`: the finite-element flow and heat solver.
- `crates/flow-post`: derived fields, streamlines, probes, integrated values, exports.
- `crates/flow-verify`: the verification cases, as tests and as the in-app report.
- `crates/flow-cli`: run a `.dflow` from the command line.
- `src-tauri`: the desktop shell (commands, files, updater).
- `ui/src`: the window (SolidJS + TypeScript). It shows what the engine computes and sends edits back as operations; it never calculates results.
- `docs/MEMORY.md`: architecture, decisions, roadmap and a log of every session. Read it first.

## Before you send a change

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace            # engine tests, including the hardening tests
npm run check:ui                  # type-check the window
npm run test:ui                   # UI tests (Playwright)
cargo deny check                  # licences, advisories, sources (cargo install cargo-deny)
```

Keep code compact and in the style around it. A new engine feature comes with a test against a known answer; a new window feature with a UI test where it can.

Licences: only MIT, BSD, Apache-2.0 or MPL-2.0 code may be linked into the app; never GPL or AGPL. Read a dependency's licence before adding it and record it in `NOTICE`.

Public words (README, help, comments, commits): describe what Datum Flow does; never name or compare with commercial products (`tools/commercial-names.txt` lists the names the checks look for).

By contributing you agree your work is released under the MIT licence.
