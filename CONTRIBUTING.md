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
local run does not cover the others. CI does.

MSRV is **Rust 1.95** (`rust-version` in the workspace `Cargo.toml`). The
workspace is `edition = "2024"`, `resolver = "3"`.

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
