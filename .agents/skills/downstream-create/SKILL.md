---
name: downstream-create
description: Scaffold a new beet downstream repo (beet_<name>, a sibling of this checkout depending on the beet facade by path) from the template, register it with downstream-sync and run the first sync. Use when starting a repo that builds on beet.
---

# Downstream Create

A downstream is a separate git repo building on beet through a path dependency on the `beet` facade, inheriting beet's conventions through `downstream-sync`. Read that skill first: what a downstream inherits, the `AGENTS.md` contract and where its skills come from live there and are not repeated here. This skill writes the repo-shaped part, the files `downstream-sync` deliberately leaves alone, in their initial shape.

## The shape

`template/` is the scaffold, a single package: a lib whose `@Name@Plugin` registers the types the repo defines, a `cli` feature gating the repo's own beet binary (`BeetPlugins` + `@Name@Plugin` + `LaunchPlugin`, see `crates/beet-cli/README.md`, Downstream binaries) and the `main.bsx` it serves, one `CliServer` root with a `Router` child.

- Placeholders: `@name@` the snake-case suffix (the repo is `beet_@name@`), `@Name@` its Pascal case, `@description@` one line, `@fmt_toolchain@` the nightly pin read from beet's justfile.
- A repo that grows more than one concern splits into a workspace, `crates/*` re-exported by a root facade crate (`beet_atproto`); the template does not pre-empt that.
- The template's `AGENTS.md` header carries the deltas every downstream has; a repo's own commands, target quirks and conventions join them above the markers.
- Licensing matches beet's: the package declares `MIT OR Apache-2.0` and `downstream-sync` copies the two license files in.

## Run it

```sh
BEET=/home/pete/me/beet
name=connect                    # the suffix: the repo is beet_$name
description="A beet downstream" # one line, the package description
repo=/home/pete/me/beet_$name
pascal=$(sed -E 's/(^|_)([a-z])/\U\2/g' <<< "$name")
fmt_toolchain=$(sed -nE "s/^fmt-toolchain := '(.*)'/\1/p" "$BEET/justfile")
[ -e "$repo" ] && echo "$repo exists" >&2 || {
	cp -r "$BEET/.agents/skills/downstream-create/template" "$repo"
	find "$repo" -type f -exec sed -i \
		-e "s/@name@/$name/g" -e "s/@Name@/$pascal/g" \
		-e "s|@description@|$description|g" -e "s/@fmt_toolchain@/$fmt_toolchain/g" {} +
	git -C "$repo" init -q
}
```

Then:

1. Add `$repo` to the repo list and the `DOWNSTREAM` array in `downstream-sync/SKILL.md` and run that skill: it fills the `AGENTS.md` block, links `CLAUDE.md`, copies `rustfmt.toml` and the `LICENSE-MIT.txt`/`LICENSE-APACHE.txt` pair.
2. Verify from `$repo`: `just test` passes (the harness runs with no cases) and `just cli --help` lists the entry's routes.
3. Nothing is committed, here or there; the user creates the remote (`mrchantey/beet_<name>`).
