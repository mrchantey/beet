# The Works

This task is for maximally ensuring the project is running smoothly.

## 1. run core tests

run this skill to completion: `.agents/skills/testing/run-tests.md`

## 2. run all tests

run this skill to completion: `.agents/skills/testing/run-tests.md` but replace `test-core` with `test-all`

## 3. run examples

run this skill to completion: `.agents/skills/testing/run-examples.md`

## 4. verify rsx_site

`rsx_site` (the typed-authoring example) is a workspace member but is excluded from the `test-core`/`test-all` recipes, and its `src/codegen/` route modules are gitignored (generated, not committed). So a stale or missing codegen breaks any workspace-wide build, yet nothing above catches it. Verify it explicitly:

1. regenerate the route codegen (compiles without the generated modules, so it needs `--no-default-features`):
   ```sh
   cargo run -p rsx_site --no-default-features --features codegen
   ```
2. render a route to the terminal and confirm the page body appears (not a `not found` / available-routes listing):
   ```sh
   cargo run -p rsx_site --features=cli -- counter
   ```

Both must exit 0, and step 2 must show the page content wrapped in the site shell. Fix any breakage (a common cause is generated route code drifting from a `beet_router` API change) and rerun.
