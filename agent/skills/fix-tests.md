## Fix tests

## Context

Always use `tail` when running tests to preserve context.

## Hanging Tests

Always use `timeout` when running tests to catch hanging tests.
Run an exponential backoff timeout, starting with two minutes, to catch hanging tests
once you find a hanging test, shorten the timeout as much as possible to speed iteration cycles.
When debugging hanging tests always pass the `log-runs` flag to detect the tests that dont finish, ie: `timeout 120 cargo test -p beet_core --lib -- --log-runs`. 
This will help you determine which test didnt complete

## Compile Errors

these sweeping tests perform large compilations and sometimes the mold linker trips up. try running again, and if it still trips up sometimes incrementing the workspace and crates version, ie find-repplace Cargo.toml `0.0.9-dev.6`, ..dev.7, will unstuck it.
A classic linker error is:
`help: you can increase rustc's stack size by setting RUST_MIN_STACK=..`
this can usually be fixed by running again

## Instructions

The timeout provided is negotiable, adjust as needed.

Run `timeout 300 just test-core`
If you encounter an error, isolate it and run again, ie if the error is in beet_core lib:
`timeout 120  cargo test -p beet_core --all-features --lib -- test_name`
Fix the error using a subagent, then run the crate again:
`timeout 120 cargo test -p beet_core --all-features`
When thats clear, run the full suite again
`timeout 300 just test-core | tail`
After the first complete pass, run fully again, checking for warnings. Fix any warnings encountered

## Success

Success means that `just test-core` passes without warnings, the whole command. not just each crate working individually.

Upon completion provide a comprehensive summary of what was changed.
