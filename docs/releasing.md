# Versioning and releasing

How a Minerva version is numbered, made and published as a GitHub Release. Follow it top to bottom for every release.

## Versioning

- Versions are `MAJOR.MINOR.PATCH`. Before 1.0:
  - **MINOR** (0.6.0 → 0.7.0) for anything a player notices as new or different: features, a changed panel, a new data source.
  - **PATCH** (0.2.0 → 0.2.1) for fixes only, with nothing new to learn.
- The version lives in four files, which always agree:
  - `package.json` (`"version"`)
  - `src-tauri/tauri.conf.json` (`"version"`): the installers' version, and the one About & Credits shows
  - `src-tauri/Cargo.toml` (`version`)
  - `crates/minerva-core/Cargo.toml` (`version`)

  `Cargo.lock` follows when Cargo next builds.
- The tag is `vX.Y.Z`, on the merge commit that brings the version into `main`. The release is titled `Minerva X.Y.Z`.
- A published version is never moved or rebuilt. If something is wrong with it, release the next PATCH.

## Before you start

- **`main` holds everything for the release, and passes its checks:**
  - `cargo test`
  - `cargo test -p minerva --lib`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `pnpm build`
- **The live data still reads:** `cargo run -p minerva --example probe osms` reads OSMS Data Explorer's live set in full.
- **A smoke test against the game** (`pnpm tauri dev`): the map is recognised; the Map section shows its picture; Search sets a Hunting Target, then a Destination, and Navigation shows the route to each; an item picked in Search shows its card beside the panel; a Mob Card opens; Item Lookup opens a page.
- **The site describes this release** (`site/`, published from the tag in step 6): every change a player notices is on it, and nothing that's gone. While the game is open for the smoke test, retake each shot under Retake in `site/screenshots/README.md` as it says, set the version they were taken on, empty the list, and merge them into `main`. `pnpm site:build` passes, and `pnpm site:dev` reads right.
- **The credits are current:**
  - If dependencies were added, removed or upgraded, regenerate the crate table in `THIRD-PARTY-NOTICES.md` (how is described there).
  - If a bundled asset or data source changed, update About & Credits in `setup.html` to match.
- **The Map Leaderboards API is live** before the first release that uploads to it: its one-time setup and deploy are in `docs/map-leaderboards-setup.md`. `curl https://owlofminerva.app/api/maps` answers with JSON.
- **Tools:** `gh` is signed in to an account that can push to `itechify/minerva`. Cargo is on `PATH`; in Git Bash, `export PATH="$HOME/.cargo/bin:$PATH"`.

## 1. Bump the version

From an up-to-date `main`, on a branch named for the version:

```bash
git fetch origin
git switch -c t3code/version-X.Y.Z origin/main
```

1. Change the version in the four files listed above.
2. Run `cargo check -p minerva` so `Cargo.lock` follows.
3. Check that `git diff --stat` shows those four files and `Cargo.lock`, with one line changed in each, two in `Cargo.lock`.
4. Commit:

```bash
git commit -am "Version X.Y.Z"
```

## 2. Merge into main

Merge without fast-forwarding, so the version has its own merge commit to tag, then push:

```bash
git switch main && git pull --ff-only origin main
git merge --no-ff t3code/version-X.Y.Z -m "Merge version X.Y.Z"
git push origin main
```

## 3. Build the release files

On the merge commit, with nothing uncommitted:

```bash
pnpm install --frozen-lockfile
pnpm tauri build
```

Tauri builds the optimised app and both installers. It downloads the NSIS and WiX tools the first time. The files land under `target/release/`, at the repo root, because the repo is a Cargo workspace:

| Built file | Upload it as |
| --- | --- |
| `target/release/bundle/nsis/Minerva_X.Y.Z_x64-setup.exe` | as is |
| `target/release/bundle/msi/Minerva_X.Y.Z_x64_en-US.msi` | as is |
| `target/release/minerva.exe` | `minerva-X.Y.Z-portable.exe` (copy and rename) |
| `THIRD-PARTY-NOTICES.md` | as is |

Before uploading, install the setup `.exe` over the previous version and check that it starts: About & Credits shows `X.Y.Z`, and settings and Personal Bests carried over.

## 4. Tag

```bash
git tag -a vX.Y.Z -m "Minerva X.Y.Z"
git push origin vX.Y.Z
```

## 5. Write the release notes

Write them for players, not developers, in the shape of the previous releases. A feature's "More on the site" links to its row on the home page (`#map`, `#item-card`), or to the FAQ question that answers it (`/faq/#zoom`). Put them in a file outside the repo, say `notes.md`:

```markdown
<One sentence: what this version gives the player.>

> Minerva isn't made, endorsed or supported by NEXON. Using any third-party tool with MapleStory is at your own risk.

## Changes

- **<Feature in bold.>** <What the player sees and does, in the panel's own words.> [More on the site](https://owlofminerva.app/#<feature>)
- <Smaller changes, one line each.>

Your settings, hotkeys and Personal Bests carry over from <previous version>.

## Download (Windows 10/11, 64-bit)

| File | What it is |
| --- | --- |
| `Minerva_X.Y.Z_x64-setup.exe` | Installer (recommended). Installs for your user over earlier versions, with a Start menu entry and an uninstaller. |
| `Minerva_X.Y.Z_x64_en-US.msi` | The same, as an MSI package (installs for all users, so it asks for admin rights). |
| `minerva-X.Y.Z-portable.exe` | No install: run it from any folder. Needs the Microsoft Edge WebView2 runtime, which Windows 11 already has. |

Quit the running Minerva first (its owl in the notification area: **Quit Minerva**). The files aren't code-signed, so Windows SmartScreen may say it "protected your PC": choose **More info**, then **Run anyway**.

Map and mob data, map pictures and mob animations: [OSMS Data Explorer](https://osmsdataexplorer.com). Pages: [NiaMeowDB](https://meowdb.com). Title and item recognition: [ocrs](https://github.com/robertknight/ocrs) (model under CC BY-SA 4.0). Typeface: Maplestory by NEXON. See THIRD-PARTY-NOTICES.md, attached, and About & Credits in Help & Setup.
```

To list what changed since the last release, run `git log --first-parent vPREVIOUS..vX.Y.Z` for the merges and `gh pr view <n>` for each PR.

## 6. Publish

```bash
cp target/release/minerva.exe minerva-X.Y.Z-portable.exe
gh release create vX.Y.Z --verify-tag --title "Minerva X.Y.Z" --notes-file notes.md \
  target/release/bundle/nsis/Minerva_X.Y.Z_x64-setup.exe \
  target/release/bundle/msi/Minerva_X.Y.Z_x64_en-US.msi \
  minerva-X.Y.Z-portable.exe \
  THIRD-PARTY-NOTICES.md
```

The newest release becomes Latest, which the README's download link points to. Publishing it runs the Site workflow (`.github/workflows/site.yml`), which builds the site from the tag and publishes it to <https://owlofminerva.app/>.

Then check the release:
- `gh release view vX.Y.Z` lists the four files.
- The release page reads correctly.
- Downloading the setup `.exe` from it and installing works.
- The Site workflow passed (`gh run list --workflow site.yml --limit 1`), and the site says it describes X.Y.Z, its Download button fetching that version's setup `.exe`.

## If a release goes wrong

- **Before anyone could download it** (minutes after publishing):
  1. `gh release delete vX.Y.Z --cleanup-tag --yes` removes the release and its tag. If the site was already published from it, publish it again from the previous release: `gh workflow run site.yml` builds from the latest release's tag. But if that tag is older than the Map Leaderboards API, publish from `main` instead (`gh workflow run site.yml -f ref=main`): an older tag deploys a Worker without the API (`docs/map-leaderboards-setup.md`).
  2. Fix the problem on `main`.
  3. Tag and publish again from step 3.
- **After that:** leave the release, and publish `X.Y.(Z+1)` with the fix.
