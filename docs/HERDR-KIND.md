# Making `herdr agent start --kind rpi` work

The extension alone cannot do this. Herdr's own docs ("Agents", "Add Herdr
support") say new agent kinds need a Herdr binary update: process detection,
labels, and the `--kind` list live in `crate::detect::Agent` (`src/detect/mod.rs`).

## Policy: do not open a PR

`herdrdev/herdr` `CONTRIBUTING.md` closes unsolicited implementation PRs
automatically unless the author is a maintainer or is listed in
`.github/APPROVED_CONTRIBUTORS`. Feature requests go to **GitHub Discussions**,
not issues. So: post a Discussion first; only submit the patch below if
maintainers invite it.

## Discussion text (short)

> rpi (https://github.com/<owner>/rpi-herdr adapter) reports state to Herdr with
> `--source rpi --agent rpi`, so sidebar and `agent list` work. But on 0.9.1,
> `herdr agent prompt` returns `not an active named agent` and `agent explain`
> shows no detected agent label, and `agent start --kind rpi` is not accepted.
> Could Herdr add `rpi` as a known agent kind?

## Patch sketch (for maintainers or an approved contributor)

In `src/detect/mod.rs`, mirroring an existing agent such as `Kilo`:

1. Add `Rpi` to `enum Agent`, and to `Agent::ALL` (the `--kind` list; array
   length 24 becomes 25). Add to `SCREEN_MANIFEST_AGENTS` only if shipping a
   screen manifest; otherwise leave it state-reported only, like `Omp`.
2. `agent_label`: `Agent::Rpi => "rpi"`; the second label match (~line 179) too.
3. `identify_agent`: `"rpi" => Some(Agent::Rpi)` (process name `rpi`).
4. Update the identify/label tests next to `kilo` (~lines 1020, 1067, 1107).
5. If a manifest is shipped: `src/detect/manifests/rpi.toml`.
6. Docs: add rpi to the "Agents" table in `docs/next/website/src/content/docs/agents.mdx`.

Never map the label `pi`: rpi is a separate agent and `pi` already exists.
