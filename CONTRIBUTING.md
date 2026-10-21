# Contributing

Thanks for taking a look. The project is in early development. The plan in
[docs/plan/](docs/plan/00-index.md) and its milestones
([15-milestones.md](docs/plan/15-milestones.md)) show where help fits.

## Commit style

This repo follows
**[Conventional Commits](https://www.conventionalcommits.org/)**.

```
type(scope): short summary

Longer body explaining the WHY, not the WHAT.

Closes #123
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `perf`,
`ci`, `build`. Scope is optional; use the crate without its prefix
(`feat(wm): …`, `fix(sources): …`).

Breaking changes: append `!` after type (`feat(config)!: ...`) and explain in
the body.

## Development

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same on Linux, macOS and Windows, plus `cargo nextest`,
`cargo deny`, `cargo audit` and `cargo machete`. Platform-gated code
(`#[cfg(windows)]`, `#[cfg(target_os = "macos")]`,
`#[cfg(target_os = "linux")]`) is only compiled on its platform, so a green
local run does not cover the others. CI does;
`cargo clippy --target x86_64-pc-windows-gnu` and
`--target aarch64-apple-darwin` catch most of it locally first.

Some tests need what only a desktop has, and are ignored unless asked for:

```sh
# the Linux sources against this machine's UPower, power-profiles-daemon,
# NetworkManager, BlueZ, timedate1 and sound server (one steps the default
# sink's volume by 5% down and back, one starts and stops a Bluetooth scan,
# one switches the power profile and back)
cargo test -p lyr-sources --test system_services -- --ignored --nocapture

# the Hyprland backend against a Hyprland started for it (it logs that one
# out), never the session's own
LYR_E2E_HYPRLAND=<signature> cargo test -p lyr-wm --lib a_real_hyprland -- --ignored

# lyr's defaults against the waybar config they port
LYR_WAYBAR_CONFIG=path/to/config.jsonc cargo test -p lyr-config the_defaults_match -- --ignored
```

MSRV is **Rust 1.98** (`rust-version` in the workspace `Cargo.toml`), crcbl's.
The workspace is `edition = "2024"`, `resolver = "3"`.

## Repo layout

```
apps/
└── lyr/            # bin: `lyr`
crates/
├── lyr-config/     # lib: TOML config + defaults
├── lyr-core/       # lib: state, layout, draw lists (no platform code)
├── lyr-render/     # lib: drawing through crcbl
├── lyr-shell/      # lib: bar and popup windows over crcbl-shell
├── lyr-wm/         # lib: workspace backends
└── lyr-sources/    # lib: module data sources
```

The binary stays thin; logic belongs in a library crate where it can be tested
without a display. `lyr-core` and `lyr-config` must not contain platform code.

## Issues + PRs

- Pick an open issue, comment that you're starting.
- Branch off `main`, name like `feat/tray-menus` or `fix/clock-tick`.
- Squash on merge unless commits are individually meaningful.

## License

By contributing, you agree your changes are MIT-licensed.
