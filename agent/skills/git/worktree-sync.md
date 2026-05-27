# Worktree Sync

Sync this worktree with the main repo at `~/me/beet`. Do not delete this worktree.

**Pull** (default): merge commits from the main worktree into this one.

**Push**: merge this worktree upstream into main.

For either direction:
- Carefully resolve all conflicts.
- If asked to test, run a sweep and fix any issues:
  ```sh
  just test-core | tail
  ```
  Commit the merge and resolved conflicts, but leave the test fixes unstaged for review.
