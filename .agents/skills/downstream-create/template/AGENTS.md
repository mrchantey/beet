# beet_@name@

@description@

## Context

This is a downstream project of the primary beet project at `/home/pete/me/beet`, depended on by path. Beet's conventions are inherited through the synced block at the bottom of this file (refreshed by beet's `downstream-sync` skill); where this header conflicts with the block, the header wins. You have permission to make changes as required, but do not commit them, so the user can review.

Downstream deltas from the inherited conventions:

- Depend on the `beet` facade, never the individual crates: external `#[action]`/`#[template]` macro expansions emit `beet::` paths, which only resolve against the facade. Direct bevy paths go through `beet::exports::bevy`.
- Tests use `#[beet::test]` and `beet::test_main!()`, the facade re-exports of the beet_core harness; the `test_main!` invocation is `#[cfg(test)]` gated so a plain build does not need the facade's `testing` feature.
- Behavior beet should own goes upstream in `../beet`, left unstaged there for review.

## Commands

- `just cli <args>`: this repo's beet binary, the stock runner plus `@Name@Plugin`, serving `main.bsx`
- `just test`: the native suite
- `just fmt`: the pinned nightly format, never `cargo fmt`

<!-- beet:sync:begin, beet's AGENTS.md refreshed by the downstream-sync skill, do not hand-edit -->
<!-- beet:sync:end -->
