# Worktree Sync

Sync this worktree with its upstream branch in the main repo at `~/me/beet`, in **both directions**. Do not delete this worktree and do not create a branch for it.

## How these worktrees work

This is a Zed-style worktree: a separate cloned directory in **detached HEAD** (no branch), so parallel agents can each own a directory without competing for branch names. Keep it detached.

Think of the worktree as `<upstream-branch> + your own commits on top`. The upstream branch (eg `beet_stack`) is checked out in the main repo. It does not matter how long-lived the worktree is, as long as it stays a clean linear extension of upstream.

## The duplication pitfall

Rebasing or replaying commits you do not own mints new hashes for identical changes. Git then sees two lineages with the same content, so every later sync conflicts or stacks up duplicate commits (same message, different hash). The rule that avoids this: only ever rebase YOUR commits, and only ever fast-forward the upstream branch. Never `git merge` the full divergent history in either direction.

## Clean sync

Run from this worktree. Substitute the upstream branch name for `<up>`.

1. Make sure the local upstream branch is current with its origin.
2. Confirm the main repo working tree is clean (the fast-forward in step 4 touches it).
3. **Pull**: replay only your commits onto the latest upstream, staying detached:
   ```sh
   git rebase <up>
   ```
   This brings in all upstream work and lands your commits on top. Resolve any conflicts carefully.
4. **Push**: fast-forward the upstream branch to this worktree's HEAD:
   ```sh
   git -C ~/me/beet merge --ff-only $(git rev-parse HEAD)
   ```
5. Confirm sync: `git rev-list --left-right --count HEAD...<up>` reports `0	0`.

Never push to `origin` unless asked.

## If history is already tangled

Symptom: many commits in `<up>..HEAD` share messages with upstream commits (duplicates from a past rebase). Recover by replaying only the genuinely unique commits:

1. Back up first: `git tag backup/<name> HEAD`.
2. Inspect `git log --oneline <up>..HEAD` and find the last duplicate commit (everything below your real work).
3. Replay only your work onto upstream: `git rebase --onto <up> <last-duplicate-commit>`.
4. Verify no content changed: `git diff HEAD backup/<name>` is empty.
5. Then fast-forward upstream as in step 4 above.

## Testing

If asked to test, run a sweep and fix any issues:
```sh
just test-core | tail
```
Leave the test fixes unstaged for review.
