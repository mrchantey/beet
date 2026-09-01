---
name: git-sync-all
description: Sync the main branch with every listed worktree branch in both directions, applying the git-worktree-sync skill to each. Use when asked to sync all worktrees.
---

# Sync All

We will now sync the main branch with the specified work tree branches.

Apply the `git-worktree-sync` skill to the following worktrees.

- Main Branch `~/me/beet`
- Merge: changes in these worktrees are intended to be merged
	- `~/me/worktrees/beet/rendering/beet`
	- `~/me/worktrees/beet/apps/beet`
- Ignore: experimental and not to be merged
	- any other not explicitly listed above, ie coding, web

Do Not run cargo fmt.

Naturally, if you need to make changes to resolve conflicts, then after those conflicts are resolved, you'll need to re-sync with those child work trees so that everything is in lockstep finally.

Use subagents where appropriate, ie to investigate and fix a nasty bug that may blow up your context.

If asked to do the works, run the `test-the-works` skill after synchronizing.
