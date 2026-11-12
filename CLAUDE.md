# Datum Flow: notes for Claude

Read `docs/MEMORY.md` first. It is the project's memory: architecture, decisions, roadmap and a log of every session. The family design guide is `FAMILY.md` in `Edgar-Mendonca/Datum` (read-only from here; `docs/FAMILY.md` links to it). The brief this app started from is `briefs/DATUM-FLOW.md` in the same hub repo.
At the end of every session that changes anything, update `docs/MEMORY.md`:
- tick the roadmap
- add a session-log entry
- record any new decisions
- refresh the Files table if files moved

User preferences:
- Rust and Tauri 2, small and efficient code, Windows first. The same look, ribbon, help, printing and updater as every Datum app (FAMILY §2–9); copy from Datum Plan (the shell) and Datum Mesh (the closest relative) rather than reinventing. Every copy becomes Flow's own code: apps never link each other.
- Ship in small numbered releases (0.1.0, 0.1.1, …), in batches. Every release appears in the in-app updater.
- **All physics lives in the Rust engine** (`crates/`, pure Rust, unit-tested). The window never calculates results: it shows what the engine sends and sends edits back as `Op`s.
- Our own solver, written from published methods (Taylor–Hood finite elements; Picard and Newton). Outside code only as infrastructure (spade for triangulation, faer for sparse LU).
- The engine and the `.dflow` file use SI base units (m, kg, s, K, Pa, W). Display units come from `flow_core::units` (sent to the window in `out.units`).
- Do not stop to ask questions the brief already answers. If something is unclear, choose the simplest sensible option, write it in `docs/MEMORY.md` (Decisions) and carry on.
- Public words (the repo is public: README, CHANGELOG, NOTICE, site.json, help, the window, docs, this file, code comments, commits): never name, compare with, or say the app copies or replaces a commercial simulation, CFD, CAD or office product (FAMILY §13). Open-source projects and standards may be named where they are used (spade, faer, VTK's file format, ParaView as a free viewer; later SU2 and OpenFOAM as optional external solvers). Benchmark papers are cited by author and year. The names not to use are listed in `tools/commercial-names.txt`; a UI test fails if one appears in the window or the source.
- Licences: read each dependency's licence before adding it; MIT, BSD, Apache-2.0, MPL-2.0 only in the app. **Never** link GPL or AGPL code. **Never** use Shewchuk's Triangle or its wrappers (its terms forbid commercial distribution). **FluidX3D may never be used** (its licence bans commercial use). LGPL only as a separate, replaceable DLL with its source offered. Record every dependency in `NOTICE`; `cargo deny check` runs in CI.

Quality bar (brief §3 and §5):
- Every solver path has a test against a known answer; each is a row of the verification report (`crates/flow-verify`, written to `docs/VERIFICATION.md` and `docs/verification/`, shown in Help › Verification). Any FAIL blocks a release.
- **Never cut** the verification report, the boundary checks, mass conservation, or the cylinder and cavity benchmarks.
- Reference values are keyed in from published papers with a citation. `docs/VERIFICATION.md` says for each one whether it was checked against the original paper or only a secondary source; say so honestly.
- Report only checks that actually ran; say what was not verified.

Checks before pushing (run locally first; CI is the safety net):
- `npm run build:ui` first (the shell embeds `ui/dist`)
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (includes the `hardening` tests and the quick verification cases)
- `cargo test --workspace --release -- --include-ignored` before a release (the full verification suite; minutes)
- `npm run check:ui`, then `npm run test:ui` (dumps the engine fixture, builds the window, runs the Playwright checks)
- `cargo deny check`
- A change you can see: check it in the real app (`cargo build -p datum-flow && tests_ui/realapp.sh`) and add a UI check for every bug

Build budget (FAMILY §9): CI runs on pushes to `main` and on pull requests only, with `concurrency` cancel-in-progress and `paths-ignore` for docs. The fast job is Linux, debug, 30 minutes at most. Heavy work (release-mode solver tests, the full verification suite, the Windows build, UI tests) runs nightly when something changed, on manual dispatch, and in the release workflow. No macOS jobs.

Releasing:
1. Bump `version` in the root `Cargo.toml` (`[workspace.package]`; every crate and the installer read it) and in `src-tauri/tauri.conf.json` if it carries one.
2. Add a CHANGELOG section (`## x.y.z — date`), in plain language.
3. Regenerate the verification report (`cargo run --release -p flow-verify --bin report`) and keep `docs/site.json` true.
4. Run `release.yml` from the Actions tab (workflow_dispatch; tag pushes are blocked from Claude sessions). It runs the full verification suite, builds `DatumFlow-x.y.z-setup.exe` and `SHA256SUMS.txt`, attests build provenance, creates the tag and publishes.
