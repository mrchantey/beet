---
name: git-sync-all
description: Sync the main branch with every worktree in both directions, applying the git-worktree-sync skill to each in turn, then levelling them. Use when asked to sync all worktrees.
---

# Sync All

Sync `~/me/beet`'s `main` with every worktree, in both directions, by applying the `git-worktree-sync` skill to each.

## Which worktrees

**Enumerate them, never hard-code them**: `git -C ~/me/beet worktree list`. A hard-coded list goes stale the first time a worktree is added or renamed, and then the sync silently skips real work. Every worktree it lists is in scope; treat none as experimental unless the operator says so for that run.

## One at a time, then level

Sync them **sequentially**, never in parallel: each one's last act is a fast-forward of `main`, and two of those at once race for the same ref.

Sequential syncing has a consequence worth knowing before you report success. Each worktree is level with `main` at the moment ITS sync finishes, and then the next one pushes on top, so by the end only the last worktree is actually level. Finish with a **levelling pass**: `git -C <dir> checkout --detach <final main sha>` for every worktree but the last. Those are pure fast-forwards, since each worktree's own commits are already in `main` by then, so no hash is minted and nothing can duplicate.

Only then does `git rev-list --left-right --count HEAD...main` report `0	0` from every worktree, which is the real end condition.

## Rules

- Do not run `cargo fmt`; formatting is `just fmt`.
- Keep every worktree detached and never push to `origin` unless asked.
- If resolving a conflict changes content, re-sync the worktrees that already finished, so everything ends in lockstep.
- Use subagents where appropriate, ie to investigate a nasty bug that would blow up your context.
- If asked to do the works, run the `test-the-works` skill after synchronising.
