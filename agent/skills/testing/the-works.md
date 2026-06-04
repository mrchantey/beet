# The Works

This task is for maximally ensuring the project is running smoothly.

## 1. run core tests

run this skill to completion: `agent/skills/run-tests.md`

## 2. run all tests

run this skill to completion: `agent/skills/run-tests.md` but replace `test-core` with `test-all`

## 3. run examples

run this skill to completion: `agent/skills/run-examples.md`

## 4. verify beet_site

`beet_site` is a workspace member but is excluded from the `test-core`/`test-all`
recipes, and its `src/codegen/` route modules are gitignored (generated, not
committed). So a stale or missing codegen breaks any workspace-wide build, yet
nothing above catches it. Verify it explicitly:

1. regenerate the route codegen (compiles without the generated modules, so it
   needs `--no-default-features`):
   ```sh
   cargo run -p beet_site --no-default-features --features codegen
   ```
2. render a blog post to the terminal and confirm the post body appears (not a
   `not found` / available-routes listing):
   ```sh
   cargo run -p beet_site --features=cli -- blog post-1
   ```

Both must exit 0, and step 2 must show the post content wrapped in the site
shell. Fix any breakage (a common cause is generated route code drifting from a
`beet_router` API change) and rerun.
