# 14 — CI and release

## Template

tmxr's pipeline (which mirrors hrdr's) is the template. lyr copies its shape:
checks now, release jobs at the first release.

## Phase 1 — now (scaffold)

Checks only, no deploy steps. `.github/workflows/ci.yml`, triggered on
`pull_request`, `push` to `main` and `v*` tags, and `workflow_dispatch`:

| Job       | Runs on                  | What                                                                              |
| --------- | ------------------------ | --------------------------------------------------------------------------------- |
| `fmt`     | ubuntu                   | `cargo fmt --all --check`                                                         |
| `clippy`  | ubuntu / macos / windows | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`   |
| `test`    | ubuntu / macos / windows | `cargo nextest run --workspace --all-features --locked --no-fail-fast` + doctests |
| `smoke`   | ubuntu / macos / windows | build, `lyr --version`, `lyr --help`                                              |
| `machete` | ubuntu                   | unused dependencies                                                               |
| `deny`    | ubuntu                   | `cargo deny --all-features check` (licenses, bans, sources, advisories)           |
| `audit`   | ubuntu                   | `cargo audit`                                                                     |

Conventions carried over from tmxr: `RUSTFLAGS=-D warnings`,
`CARGO_TERM_COLOR=always`, `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24`, a concurrency
group that cancels superseded non-tag runs, `fail-fast: false` matrices,
`actions/checkout@v7`, `actions-rust-lang/setup-rust-toolchain@v2` with
`toolchain: stable` + `rustup update`, `Swatinem/rust-cache@v2`, and
`taiki-e/install-action` for nextest, machete, deny and audit.

Also: `.github/workflows/cron.yml` (`cargo deny` on Mondays at 06:00 UTC),
`.github/dependabot.yml`, `CODEOWNERS`, the PR and issue templates, and
`.config/nextest.toml`.

### When crcbl arrives (M1)

- **Git dependency:** `deny.toml` gains `allow-git` for
  `https://github.com/kryptic-sh/crcbl` until crcbl is on crates.io, as tmxr did
  for its crossterm fork. The entry is removed when crcbl publishes.
- **Toolchain:** lyr's `rust-version` follows crcbl's (1.98 since lyr took on
  `crcbl-shell`), and is raised with each crcbl bump that raises it.
- **Linux system packages:** crcbl `dlopen`s libwayland and the Vulkan loader at
  runtime, so they are not needed to build. Whether the Linux runner needs any
  `-dev` packages (crcbl's own docs mention `libasound2-dev` for its audio
  crate, which lyr does not use) is checked when the dependency is added, not
  assumed.
- **Headless render tests** run on the ubuntu runner only
  ([13](13-testing.md#2-integration-tests)).

## Phase 2 — first release (M11)

Add tmxr's release jobs, renamed for lyr:

- `build`: the target matrix (Linux gnu and musl for x86_64 and aarch64, Windows
  MSVC, macOS aarch64 and x86_64); the tag ↔ `Cargo.toml` version check;
  `.tar.gz` / `.zip` + `.sha256`; `.deb` / `.rpm` for gnu targets.
- `install-packages`, `publish-github-release`, `publish-crates` (topological
  `ORDER` with the drift check), `aur-bin`, `brew-tap`, `scoop-bucket`,
  `alpine`, and the `tag-release-status` aggregate that fails the tag run if any
  job did not succeed.
- `pkg/` templates: `aur/`, `homebrew/`, `scoop/`, `alpine/`.
- The binary gains `--completions <shell>` and `--man`.

Before the first tag:

- **Every crcbl crate lyr uses is on crates.io**, at a release that has what lyr
  needs. `publish-crates` fails on a git dependency, so this is a hard block.
  crcbl 0.5.0 (2026-10-19) has all of it, so this holds while lyr avoids git
  pins.
- lyr is added to the allowlists of the org secrets `AUR_SSH_KEY`,
  `BREW_SSH_KEY` and `SCOOP_SSH_KEY`, and has `CARGO_REGISTRY_TOKEN`.
- Check that the `lyr` / `lyr-*` names are free on crates.io and the AUR.
- Site entry on kryptic-sh.github.io.

## Phase 3 — later

- `leak-guard` (tmxr's): run the suite with `$HOME` and the XDG directories
  pointed at an empty sentinel directory, and fail if anything lands there.
- A weekly fuzzing job for the Hyprland event parser and the config loader.
