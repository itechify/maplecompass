# AGENTS.md

## Build and test

- `cargo test`: runs the `compass-core` tests. The core is the default workspace member, so this never builds Tauri.
- `cargo clippy --workspace --all-targets -- -D warnings`: lint everything.
- `pnpm install` then `pnpm tauri build --debug --no-bundle`: build the app. `pnpm tauri dev` runs it.
- `cargo run -p maplecompass --example probe [reads] [out.png]`: read-only smoke test against a running game, with no UI.

## Code rules

- All logic goes in `crates/compass-core`, which takes plain data in and returns a `PanelState`. It has no OS, UI, network or file dependencies.
- Tests drive the core's public inputs and assert on `PanelState` only, never on internals. Every file in `samples/minimap` must be recognised; `--window` samples go through `default_title_region`, like the shell's capture.
- `src-tauri` only adapts window lookup, capture and rendering. It must follow ADR 0001: no `PrintWindow`, no messages, input, hooks or process access to the game.

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues on `itechify/maplecompass`, using the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the five default triage labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human` and `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
