# Security Policy

## Supported versions

lyr is pre-1.0. Only the latest published release receives security fixes; older
releases are best-effort.

## Reporting a vulnerability

**Do not open a public GitHub issue for security reports.**

Email `mxaddict@kryptic.sh` with:

- Affected crate(s) and version(s)
- Description of the issue and impact
- Reproduction steps or proof-of-concept
- Disclosure timeline preference

Acknowledgment within 72 hours. Coordinated disclosure window is typically 30
days from acknowledgment, extendable for complex issues.

## Threat model

lyr runs as your user and acts on your behalf:

- **Config is code.** Module actions can be `exec` commands
  ([docs/plan/05-layout-and-config.md](docs/plan/05-layout-and-config.md)).
  Treat a `config.toml` like a shell script you are about to run.
- **The control socket** (`lyr reload`, `lyr event …`) accepts commands, so it
  is restricted to your own user: a per-user directory created `0700` on Unix,
  and a pipe for the current user only on Windows.
- **Untrusted input** reaches lyr from other processes: tray item names, icons
  and menus over D-Bus, and notification counts. It is displayed, never
  executed, and icon data is size-checked before it is decoded.

## Dependencies

`cargo deny` and `cargo audit` run in CI on every push to `main` and every pull
request, checking RUSTSEC advisories along with the license and source rules in
[`deny.toml`](deny.toml). Dependabot opens grouped dependency PRs weekly (see
[`.github/dependabot.yml`](.github/dependabot.yml)).
