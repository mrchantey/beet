---
name: worktree-sync-upstream
description: Sync a detached-HEAD worktree with its upstream branch in the main repo in both directions, rebasing only owned commits and fast-forwarding upstream, with recovery steps for tangled history and helpers for prose and secrets.toml conflicts. Use from inside a worktree.
---

# Worktree Sync Upstream

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

## Resolving conflicts

Read a conflicted file against its base, `git checkout --conflict=diff3 -- <file>`: who changed what is only legible beside the base. After resolving, compile the touched crates before `git rebase --continue`, so a later commit never builds on a broken resolution.

- **Prose.** A markdown paragraph is one line, so two sides editing different sentences of it conflict on the whole paragraph. `python3 ~/me/beet/.agents/skills/worktree-sync-upstream/word_merge.py <file>...` merges each hunk word by word and leaves the rest conflicted. A clean merge can still be wrong (both sides renaming one thing), so read every result.
- **`secrets.toml`.** Each group is one age blob, so two sides that each sealed something conflict on ciphertext, and taking a side silently drops the other's records. `python3 ~/me/beet/.agents/skills/worktree-sync-upstream/secrets_merge.py` decrypts the three stages in memory and prints which side changed each record, never a value; `--write` writes the merge, resealing only what needs it, and checks every blob against its index. It stops on a field the sides changed differently, ie a credential's `grants` after a converge on each: rerun with `--take=ours|theirs`, naming the side that sealed the credential the provider holds now. Grants it leaves behind the code only make the next plain deploy refuse with `--elevated`, which is the gate working.

## Testing

If asked to test, run a sweep and fix any issues:
```sh
just test-core | tail
```
Leave the test fixes unstaged for review.
