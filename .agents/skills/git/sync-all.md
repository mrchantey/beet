# Sync All

We will now sync the main branch with the specified work tree branches.

Apply the worktree sync skill to the following worktrees.
.agents/skills/git/worktree-sync.md

- Main Branch `~/me/beet`
- Merge: changes in these worktrees are intended to be merged
	- rendering `~/me/worktrees/beet/rendering/beet`
	- apps `~/me/worktrees/beet/apps/beet`
- Ignore: experimental and not to be merged
	- coding `~/me/worktrees/beet/coding/beet`


Naturally, if you need to make changes to resolve conflicts, then after those conflicts are resolved, you'll need to re-sync with those child work trees so that everything is in lockstep finally.