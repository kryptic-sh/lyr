# 14 — CI and release

## Template

tmxr's pipeline (which mirrors hrdr's) is the template. lyr copies its shape:
checks now, release jobs at the first release.

## Phase 1 — now (scaffold)

Checks only, no deploy steps. `.github/workflows/ci.yml`, triggered on
`pull_request`, `push` to `main` and `v*` tags, and `workflow_dispatch`:

| Job          | Runs on                  | What                                                                                                                                         |
| ------------ | ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `fmt`        | ubuntu                   | `cargo fmt --all --check`                                                                                                                    |
| `clippy`     | ubuntu / macos / windows | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`                                                              |
| `test`       | ubuntu / macos / windows | `cargo nextest run --workspace --all-features --locked --no-fail-fast` + doctests                                                            |
| `smoke`      | ubuntu / macos / windows | build, `lyr --version`, `lyr --help`                                                                                                         |
| `leak-guard` | ubuntu / macos / windows | the suite with `$HOME` and the XDG directories at an empty sentinel; fails if a test writes there or into lyr's real directories (tmxr's)    |
| `sway-e2e`   | ubuntu                   | `lyr_wm::sway`'s ignored test against a headless sway with two outputs, started in a private `XDG_RUNTIME_DIR` as crcbl's Wayland suite does |
| `machete`    | ubuntu                   | unused dependencies                                                                                                                          |
| `deny`       | ubuntu                   | `cargo deny --all-features check` (licenses, bans, sources, advisories)                                                                      |
| `audit`      | ubuntu                   | `cargo audit`                                                                                                                                |

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

## Phase 2 — release (M11)

tmxr's release jobs, renamed for lyr, in `.github/workflows/ci.yml`:

- `build`, on every push that is not a pull request: Linux gnu (x86_64 and
  aarch64, glibc 2.28 through `cargo zigbuild`), Windows MSVC, and macOS aarch64
  and x86_64 at a deployment target of 11.0 (crcbl's Metal floor); the tag ↔
  `Cargo.toml` version check; `.tar.gz` / `.zip` + `.sha256`. The three it
  builds natively run `lyr --version` there; `smoke-cross` checks the other two
  archives' `.sha256` and runs `lyr --version` from them on an arm64 Linux and
  an Intel Mac runner, and publishing waits on it. No musl build: crcbl
  `dlopen`s libwayland and the Vulkan loader, which a static musl binary cannot.
- `package`: `cargo package --workspace`, every crate built from its package as
  `cargo publish` builds it, so a crate that cannot publish fails on main rather
  than stranding the crates after it on a tag.
- On a `v*` tag: `publish-github-release`, `publish-crates` (topological `ORDER`
  with the drift check; the app is `lyr-bin` on crates.io, since `lyr` there is
  another project's, and its binary is still `lyr`), `aur-bin` (`lyr-bin`),
  `brew-tap` (`kryptic-sh/homebrew-tap`), `scoop-bucket`
  (`kryptic-sh/scoop-bucket`), and the `tag-release-status` aggregate that fails
  the tag run if any job did not succeed.
- `pkg/` templates: `aur/`, `homebrew/`, `scoop/`. The AUR package depends on
  what lyr opens at run time: `wayland`, `vulkan-icd-loader`, `libxkbcommon` and
  `fontconfig`.

Not carried from tmxr: `.deb` / `.rpm` / Alpine packages, and the binary's
`--completions` and `--man` (they need `clap_complete` and `clap_mangen`, new
dependencies; see the backlog). The org secrets `AUR_SSH_KEY`, `BREW_SSH_KEY`,
`SCOOP_SSH_KEY` and `CARGO_REGISTRY_TOKEN` are visible to every repo in the org.
The site entry on kryptic-sh.github.io is the owner's.

## Phase 3 — later

- A weekly fuzzing job for the Hyprland event parser and the config loader.
