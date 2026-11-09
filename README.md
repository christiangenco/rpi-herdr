# rpi-herdr

Herdr adapter for rpi. Inside a Herdr pane it reports
`idle`, `working`, and `blocked` through `herdr pane report-agent` (source
`rpi`, agent `rpi`), so the Herdr sidebar, notifications, and `herdr agent list`
see your rpi session. Outside Herdr it does nothing. The command is `rpi`.

## Install

```sh
rpi install rpi-herdr --github <owner>/rpi-herdr
```

This uses rpi's prebuilt extension protocol: the GitHub Release tagged
`rpi-herdr-v<version>` carries per-platform archives and `rpi-extension.json`.
To build from source instead:

```sh
git clone https://github.com/<owner>/rpi-herdr && cd rpi-herdr
rpi install --path . --force
```

Restart rpi inside a Herdr pane, then check:

```sh
herdr agent list
```

## What works

| Feature | Needs |
| --- | --- |
| Sidebar state, notifications, `herdr agent list`, `herdr agent wait` | this extension only |
| `herdr agent start --kind rpi`, `herdr agent prompt` | Herdr itself must know the kind `rpi` (not yet: see [docs/HERDR-KIND.md](docs/HERDR-KIND.md)) |
| Resume after Herdr restart | Herdr 0.9.2+ and a resume command (not reported yet) |

Verified against Herdr 0.9.1: self-reporting alone does **not** make
`herdr agent prompt` work (`not an active named agent`).

## Build and release

```sh
cargo build --release          # rpi-plugin-sdk =0.21.1
git tag rpi-herdr-v0.1.0 && git push origin rpi-herdr-v0.1.0
```

The tag must match `Cargo.toml`. `.github/workflows/release.yml` builds
Linux/macOS (x86_64, aarch64) and uploads archives, then `rpi-extension.json`
last. Windows is not built.
