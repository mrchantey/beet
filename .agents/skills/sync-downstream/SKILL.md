---
name: sync-downstream
description: Refresh the downstream repos (beet_esp, beet_atproto, data-dumps) with beet's AGENTS.md marker block, the curated skill set and shared config files. Use after changing AGENTS.md, the skills, or rustfmt.toml.
---

# Sync Downstream

Beet has downstream repos (separate git repos building on beet via a path dependency). Each inherits beet's conventions rather than fracturing into its own: a verbatim copy of beet's `AGENTS.md` lives inside a marker block at the bottom of the downstream `AGENTS.md`, and the beet skills that apply to a crate built on beet are copied as-is along with shared config files. This skill refreshes all of that.

## Downstream repos

- `/home/pete/me/beet_esp`
- `/home/pete/me/beet_atproto`
- `/home/pete/me/data-dumps`

Add new spinoffs to this list (and to the `DOWNSTREAM` array below) when they are created.

## What is synced

1. `rustfmt.toml`: verbatim copy of beet's.
2. `AGENTS.md` inherited block: everything between the markers is replaced with beet's current `AGENTS.md` (the working tree version, so in-flight edits propagate):

```md
<!-- beet:sync:begin — beet's AGENTS.md, refreshed by the sync-downstream skill; do not hand-edit -->
<!-- beet:sync:end -->
```

3. `CLAUDE.md -> AGENTS.md` symlink.
4. `.agents/skills`: the skills in the `SKILLS` array below, each directory copied whole. Any directory named like a beet skill is beet-owned: the listed ones are refreshed, the unlisted ones (dropped from the list, or never in it) are removed, and everything else in the tree is the downstream's own (`beet_esp` keeps `esp-rust` there). A downstream skill must never share a name with a beet skill. Skills not in the list stay reachable at `$BEET/.agents/skills/<name>`, like every other beet-relative path in the block.

## Which skills sync

A skill syncs when an agent working on a crate built on beet would invoke it there. The list is a judgement, not a mirror; revisit it when a skill is added or its scope changes.

- Conventions the block points into: `audit-free-fns` (the free-item rule's recipe), `rendering` (the beet_ui change-render-verify loop, needed by any downstream page).
- Building on beet: `create-cli` (a downstream binary is a beet CLI).
- Per-crate hygiene: `release` (docs, native + wasm tests, examples per crate), `docs-rust-conventions`.
- Writing docs: the `docs-diataxis` family (`docs-explanation`, `docs-how-to`, `docs-reference`, `docs-tutorials`, `docs-improving`).
- Session and planning process: `all-nighter`, `phased-plan`, `write-plan`.

Not synced, as beet-repo procedures: they name beet's justfile recipes, worktrees, example set, site and website deploy (`test-run`, `test-examples`, `test-the-works`, `test-dependency-audit`, `docs-rust-sweep`, `docs-site`, `infra-deploy`, `git-sync-all`, `git-worktree-sync`, and this skill). A downstream's own equivalents (its test command, its deploy entry) belong in its `AGENTS.md` header.

## The contract

- A downstream `AGENTS.md` is its repo-specific header followed by the synced block. Where the header conflicts with the block, the header wins; downstream deltas (target quirks, path-dep notes, test attribute spellings) belong in the header, never as edits inside the block.
- Never hand-edit inside the markers; the next sync clobbers it.
- Leave all changes unstaged in every repo, including this one. Never commit.
- A downstream `AGENTS.md` missing the markers is malformed, and a missing `AGENTS.md` is the same case: the script warns and skips it. Write the header by hand with the markers at the end, never append a second copy of anything, then rerun.

## Run it

```sh
BEET=/home/pete/me/beet
DOWNSTREAM=(/home/pete/me/beet_esp /home/pete/me/beet_atproto /home/pete/me/data-dumps)
SKILLS=(
	all-nighter audit-free-fns create-cli
	docs-diataxis docs-explanation docs-how-to docs-improving docs-reference docs-rust-conventions docs-tutorials
	phased-plan release rendering write-plan
)
for repo in "${DOWNSTREAM[@]}"; do
	cp "$BEET/rustfmt.toml" "$repo/rustfmt.toml"
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
	# beet-named skill dirs are beet-owned: drop them all, copy back the listed ones
	mkdir -p "$repo/.agents/skills"
	for dir in "$BEET"/.agents/skills/*/; do
		rm -rf "$repo/.agents/skills/$(basename "$dir")"
	done
	for name in "${SKILLS[@]}"; do
		cp -r "$BEET/.agents/skills/$name" "$repo/.agents/skills/"
	done
done
```

Afterwards spot-check one downstream repo: `AGENTS.md` header intact, exactly one block with beet's current text inside it, and `.agents/skills` holding exactly the `SKILLS` list plus the repo's own skills.

## Candidates deliberately not synced

- `justfile`, `.cargo/config.toml`, `.gitignore`: repo-shaped, they drift for real reasons.
- `.github/workflows/rust_ci.yml`: revisit when the downstream repos gain remotes/CI.
