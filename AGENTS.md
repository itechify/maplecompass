# AGENTS.md

## Build and test

- `cargo test`: runs the `compass-core` tests. The core is the default workspace member, so this never builds Tauri.
- `cargo test -p maplecompass --lib`: runs the app's own unit tests, which `cargo test` leaves out.
- `cargo clippy --workspace --all-targets -- -D warnings`: lint everything.
- `pnpm install` then `pnpm tauri build --debug --no-bundle`: build the app. `pnpm tauri dev` runs it.
- `cargo run -p maplecompass --example probe [reads] [out.png]`: read-only smoke test against a running game, with no UI.
- `cargo run -p maplecompass --example probe record <out-dir> <name>=<x>,<y>,<w>,<h>@<hz> ... [--seconds <n>]`: record timestamped crops of client-area regions from a running game, for samples.
- `cargo run -p maplecompass --example probe item samples/item-tooltip "<item name>" [--desc <tag>] [--delay <s>]`: after a countdown, save the game's client area named with the item and the pointer's position, for samples.
- `cargo run -p maplecompass --example probe osms`: fetch OSMS Data Explorer's live file set and read it in full with the core's reader, reporting its counts or what didn't read. No game is needed.

## Code rules

- All logic goes in `crates/compass-core`, which takes plain data in and returns a `PanelState`. It has no OS, UI, network or file dependencies.
- Tests drive the core's public inputs and assert on `PanelState` only, never on internals. Every file in `samples/minimap` must be recognised against the map names fixture, `crates/compass-core/tests/fixtures/osms-map-names.json`; there, `--window` samples go through `default_title_region` (the probe's capture). The app captures the Title Region the player marks at setup, which `tests/setup.rs` covers with marks on the `--window` samples.
- Every file in `samples/exp-bar` must read as its name (or its recording's `readings.json`) says, or as unreadable, never as a wrong number. A frame that is ever read wrong becomes a sample there.
- Every file in `samples/item-tooltip` must read as the item its name says (or as no Item Tooltip, for `no-tooltip`), or as unreadable, never as another item. A frame that is ever read wrong becomes a sample there.
- Tests run on made-up worlds in OSMS's shape (`crates/compass-core/tests/common/osms.rs`); the map names fixture (IDs, streets, map names and regions) is the only data committed from OSMS. `probe osms` must read OSMS's live set in full. A shape OSMS ever serves that reads wrong becomes a made-up fixture with that shape.
- `src-tauri` only adapts window lookup, capture and rendering. It must follow ADR 0001: no `PrintWindow`, no messages, input, hooks or process access to the game.

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues on `itechify/maplecompass`, using the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the five default triage labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human` and `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
