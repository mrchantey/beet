---
name: test-the-works
description: Maximal project health pass. Run core tests, all tests, the example smoke set, the rsx_site verification and every downstream repo in sequence, fixing every failure and warning surfaced along the way. Use when asked to run 'the works' or fully verify the project.
---

# The Works

This task is for maximally ensuring the project is running smoothly, beet and every repo built on it.

## Ownership

Every failure, error and warning this skill surfaces is yours to fix, wherever it lives. That includes code you never touched, code another agent just committed, a feature or target combination no recipe names but a step happened to build (a downstream's wasm build, a bare-metal `testing_embedded` build), and every downstream repo in step 5.

None of these are reasons to leave something unfixed, they are exactly the findings this skill exists to produce:

- "pre-existing" or "not from my changes"
- "not a supported configuration" or "outside the recipe"
- "only a warning"
- "flaky, passed on rerun": a flake is a race, find it and fix it
- "the toolchain or tool is missing": install it, through the machine's provisioning (`/home/pete/me/arch-config`'s `justfile`) so a fresh install gets it too

The only things a report may leave standing are those that live outside every repo here and cannot be fixed from them: a third-party crate's future-incompat notice, host environment noise (a broken `~/.XCompose`), a step needing a password or physical hardware the session does not have. Each is named in the report with why it cannot be fixed here and the exact command or action that unblocks it.

## 1. run core tests

run this skill to completion: `.agents/skills/test-run/SKILL.md`

## 2. run all tests

run this skill to completion: `.agents/skills/test-run/SKILL.md` but replace `test-core` with `test-all`

## 3. run examples

run this skill to completion: `.agents/skills/test-examples/SKILL.md`

## 4. verify rsx_site

`rsx_site` (the typed-authoring example) is a workspace member but is excluded from the `test-core`/`test-all` recipes, and its `src/codegen/` route modules are gitignored (generated, not committed). So a stale or missing codegen breaks any workspace-wide build, yet nothing above catches it. Verify it explicitly:

1. regenerate the route codegen (compiles without the generated modules, so it needs `--no-default-features`):
   ```sh
   cargo run -p rsx_site --no-default-features --features codegen
   ```
2. render a route to the terminal and confirm the page body appears (not a `not found` / available-routes listing):
   ```sh
   cargo run -p rsx_site --features=cli -- counter
   ```

Both must exit 0, and step 2 must show the page content wrapped in the site shell. Fix any breakage (a common cause is generated route code drifting from a `beet_router` API change) and rerun.

## 5. verify the downstream repos

Every repo in the `downstream-sync` skill's list (`.agents/skills/downstream-sync/SKILL.md`) builds on this checkout by path, so a beet change that compiles here can still break one of them. Each must depend on `../beet` itself, never a worktree, so it verifies the tree this skill just tested. Run each repo's own test recipe (its `justfile` `test`, plus `build-wasm` where it has one) and fix what fails, upstream in beet when beet is at fault (a prelude name collision, an API narrowed below what a downstream consumes) and in the downstream when it has drifted from a deliberate beet change. A downstream's own uncommitted work is left in place and built on, never reverted.

`beet_esp` needs the Espressif Xtensa toolchain (`espup`, sourced through `~/export-esp.sh`) and `probe-rs`, both from arch-config's `install-rust`, plus a `.env` holding its Wi-Fi credentials (`cp .env.example .env`, which its build bakes in). `just check-all` builds every target without a board and fails on any warning: the on-device test binary, both firmwares, each example under its own `required-features` and the host scene types. `just test` then flashes the on-device suite to an ESP32-S3 over USB; without the board attached the report says that run is waiting on it.
