# Security

## Reporting a problem

Please report a security problem privately: use **Security › Report a vulnerability** on this repository (GitHub private advisory), or open an issue that only says you have something to report and we will contact you. Do not post details in a public issue.

## What Datum Flow does and does not do

- Models are `.dflow` files (JSON) on your own computer. Nothing is uploaded: there are no accounts and no telemetry.
- The only network call is the update check and download from this repository's GitHub Releases (HTTPS). An update installs only if the release has `SHA256SUMS.txt` and the installer matches its line exactly; the download goes to a new private folder, has a size limit, and is deleted if anything does not match.
- The window runs under a strict content security policy (no remote scripts, frames, objects or form posts) with only Tauri's core permissions; it can open only links to this repository and the Datum site (`open_url` refuses anything else).
- Opened and imported files (`.dflow`, DXF) are treated as untrusted: they are size-limited (64 MB), impossible values are reset when the model is loaded (no non-finite numbers reach the solver), and the engine is tested with damaged files, random models, random edits and deliberately hostile values. None may crash or hang the app.
- Saves are atomic: the new file is written next to the old one and then replaces it, so a crash or a full disk never leaves a half-written model. Autosave and recovery work the same way.
- CSV exports put an apostrophe before text that a spreadsheet would run as a formula (starting with `=`, `+`, `-` or `@`), so opening an exported file never runs anything.
- If anything inside the app fails, that action is stopped with a message and the model stays as it was; the details go to `crash.log` in the app's data folder (nothing is sent anywhere).
- The installer is not code-signed yet (planned for 1.0), so Windows shows a SmartScreen warning on the first install. Each release carries a build provenance attestation: `gh attestation verify DatumFlow-x.y.z-setup.exe --repo Edgar-Mendonca/Datum-Flow`.
