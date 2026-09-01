---
name: sync-downstream
description: Refresh the downstream repos (beet_esp, beet_atproto) with beet's AGENTS.md marker block, skills tree and shared config files. Use after changing AGENTS.md, the skills, or rustfmt.toml.
---

# Sync Downstream

Beet has downstream repos (separate git repos building on beet via a path dependency). Each inherits beet's conventions rather than fracturing into its own: a verbatim copy of beet's `AGENTS.md` lives inside a marker block at the bottom of the downstream `AGENTS.md`, and beet's skills tree and shared config files are copied as-is. This skill refreshes all of that.

## Downstream repos

- `/home/pete/me/beet_esp`
- `/home/pete/me/beet_atproto`

Add new spinoffs to this list (and to the `DOWNSTREAM` array below) when they are created.

## What is synced

1. `rustfmt.toml`: verbatim copy of beet's.
2. `AGENTS.md` inherited block: everything between the markers is replaced with beet's current `AGENTS.md` (the working tree version, so in-flight edits propagate):

```md
<!-- beet:sync:begin — beet's AGENTS.md, refreshed by the sync-downstream skill; do not hand-edit -->
<!-- beet:sync:end -->
```

3. `CLAUDE.md -> AGENTS.md` symlink.
4. `.agents/skills`: a mirror of beet's tree (`rsync -a --delete`), since the synced `AGENTS.md` points into it. The tree is beet-owned and clobbered on every sync; downstream-specific guidance belongs in the downstream `AGENTS.md` header, never as a skill inside the mirrored tree.

## The contract

- A downstream `AGENTS.md` is its repo-specific header followed by the synced block. Where the header conflicts with the block, the header wins; downstream deltas (target quirks, path-dep notes, test attribute spellings) belong in the header, never as edits inside the block.
- Never hand-edit inside the markers; the next sync clobbers it.
- Leave all changes unstaged in every repo, including this one. Never commit.
- A downstream `AGENTS.md` missing the markers is malformed: add the block (header first, markers at the end) rather than appending a second copy of anything.

## Run it

```sh
BEET=/home/pete/me/beet
DOWNSTREAM=(/home/pete/me/beet_esp /home/pete/me/beet_atproto)
for repo in "${DOWNSTREAM[@]}"; do
	cp "$BEET/rustfmt.toml" "$repo/rustfmt.toml"
	awk -v src="$BEET/AGENTS.md" '
		/<!-- beet:sync:begin/ { print; while ((getline line < src) > 0) print line; close(src); skip=1; next }
		/<!-- beet:sync:end/ { skip=0 }
		!skip { print }
	' "$repo/AGENTS.md" > "$repo/AGENTS.md.tmp" && mv "$repo/AGENTS.md.tmp" "$repo/AGENTS.md"
	ln -sf AGENTS.md "$repo/CLAUDE.md"
	mkdir -p "$repo/.agents"
	rsync -a --delete "$BEET/.agents/skills/" "$repo/.agents/skills/"
done
```

Afterwards spot-check one downstream repo: `AGENTS.md` header intact, exactly one block with beet's current text inside it, and `.agents/skills` matching beet's.

## Candidates deliberately not synced

- `justfile`, `.cargo/config.toml`, `.gitignore`: repo-shaped, they drift for real reasons.
- `.github/workflows/rust_ci.yml`: revisit when the downstream repos gain remotes/CI.
