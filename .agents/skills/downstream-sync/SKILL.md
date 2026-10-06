---
name: downstream-sync
description: Refresh every downstream repo (beet_atproto, beet_connect, beet_egress, beet_esp, beet_eval) with beet's AGENTS.md marker block, its MIT/Apache licenses and shared config files. Use after changing AGENTS.md, the licenses or rustfmt.toml.
---

# Downstream Sync

Beet has downstream repos (separate git repos building on beet via a path dependency). Each inherits beet's conventions rather than fracturing into its own: a verbatim copy of beet's `AGENTS.md` lives inside a marker block at the bottom of the downstream `AGENTS.md`, and beet's licenses and shared config files are copied as-is. This skill refreshes all of that; `downstream-create` scaffolds a new repo and registers it here.

## Downstream repos

- `/home/pete/me/beet_atproto`
- `/home/pete/me/beet_connect`
- `/home/pete/me/beet_egress`
- `/home/pete/me/beet_esp`
- `/home/pete/me/beet_eval`

`downstream-create` adds a new repo to this list and to the `DOWNSTREAM` array below.

## What is synced

1. `rustfmt.toml`, `LICENSE-MIT.txt`, `LICENSE-APACHE.txt`: verbatim copies of beet's, so every downstream formats like beet and is dual licensed `MIT OR Apache-2.0` like beet.
2. `AGENTS.md` inherited block: everything between the markers is replaced with beet's current `AGENTS.md` (the working tree version, so in-flight edits propagate):

```md
<!-- beet:sync:begin, beet's AGENTS.md refreshed by the downstream-sync skill, do not hand-edit -->
<!-- beet:sync:end -->
```

3. `CLAUDE.md -> AGENTS.md` symlink.

## Skills are not synced

No skill is copied into a downstream. A skill an agent working on a crate built on beet would invoke lives in the shared user-level `~/.agents/skills` (stowed from `/home/pete/me/arch-config/stow/agents/.agents/skills`), so every repo already sees it: the generic ones under their own names (`docs-diataxis`, `release`, `phased-plan`, ..), the beet-specific ones prefixed `beet-` (`beet-rendering`, `beet-create-cli`, `beet-audit-free-fns`).

Beet-repo procedures stay in `$BEET/.agents/skills`, since they name beet's justfile recipes, worktrees, example set, site and deploy; they are reachable at that path like every other beet-relative path in the block. A downstream's own equivalents (its test command, its deploy entry) belong in its `AGENTS.md` header, and its `.agents/skills` holds only its own skills (`beet_egress` keeps `download-takeout` there), never named like a shared one.

## The contract

- A downstream `AGENTS.md` is its repo-specific header followed by the synced block. Where the header conflicts with the block, the header wins; downstream deltas (target quirks, path-dep notes, test attribute spellings) belong in the header, never as edits inside the block.
- Never hand-edit inside the markers; the next sync clobbers it.
- Leave all changes unstaged in every repo, including this one. Never commit.
- A downstream `AGENTS.md` missing the markers is malformed, and a missing `AGENTS.md` is the same case: the script warns and skips it. Write the header by hand with the markers at the end (`downstream-create`'s template is the shape), never append a second copy of anything, then rerun.

## Run it

```sh
BEET=/home/pete/me/beet
DOWNSTREAM=(/home/pete/me/beet_atproto /home/pete/me/beet_connect /home/pete/me/beet_egress /home/pete/me/beet_esp /home/pete/me/beet_eval)

for repo in "${DOWNSTREAM[@]}"; do
	for file in rustfmt.toml LICENSE-MIT.txt LICENSE-APACHE.txt; do
		cp "$BEET/$file" "$repo/$file"
	done
	if grep -q '<!-- beet:sync:begin' "$repo/AGENTS.md" 2>/dev/null; then
		awk -v src="$BEET/AGENTS.md" '
			/<!-- beet:sync:begin/ { print; while ((getline line < src) > 0) print line; close(src); skip=1; next }
			/<!-- beet:sync:end/ { skip=0 }
			!skip { print }
		' "$repo/AGENTS.md" > "$repo/AGENTS.md.tmp" && mv "$repo/AGENTS.md.tmp" "$repo/AGENTS.md"
		ln -sf AGENTS.md "$repo/CLAUDE.md"
	else
		echo "$repo/AGENTS.md: missing or no sync markers, write the header by hand" >&2
	fi
done
```

Afterwards spot-check one downstream repo: `AGENTS.md` header intact, exactly one block with beet's current text inside it, and both license files present.

## Candidates deliberately not synced

- `justfile`, `.cargo/config.toml`, `.gitignore`: repo-shaped, they drift for real reasons; `downstream-create` writes their initial shape.
- `Cargo.toml`: repo-shaped; its `license = "MIT OR Apache-2.0"` (on the package, or `[workspace.package]` in a workspace) is set once, and `downstream-create`'s template carries it.
- `.github/workflows/test.yml`: revisit when a downstream gains CI.
