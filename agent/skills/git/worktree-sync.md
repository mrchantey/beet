# Worktree Sync

Sync this worktree with the main repo at `~/me/beet` in **both directions**. Do not delete this worktree.

Always check both directions and act on whichever has commits:

- **Pull**: if main has commits the worktree lacks, merge them into this worktree.
- **Push**: if this worktree has commits main lacks, merge them upstream into main's branch.

If both directions have commits, do the pull first, then the push.
If neither direction has commits, report that the worktrees are already in sync.

For either direction:
- Carefully resolve all conflicts.
- If asked to test, run a sweep and fix any issues:
  ```sh
  just test-core | tail
  ```
  Commit the merge and resolved conflicts, but leave the test fixes unstaged for review.
