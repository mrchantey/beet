# Sync All

We will now sync the main branch with the specified work tree branches.

Apply the worktree sync skill to the following worktrees.
.agents/skills/git/worktree-sync.md

- Main Branch `~/me/beet`
- Merge: changes in these worktrees are intended to be merged
	- `~/me/worktrees/beet/rendering/beet`
	- `~/me/worktrees/beet/apps/beet`
	- `~/me/worktrees/beet/presentation/beet`
- Ignore: experimental and not to be merged
	- any other not explicitly listed above, ie coding, web

Do Not run cargo fmt.

Naturally, if you need to make changes to resolve conflicts, then after those conflicts are resolved, you'll need to re-sync with those child work trees so that everything is in lockstep finally.
