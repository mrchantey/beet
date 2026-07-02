## Run tests

## Context

Always use `tail` when running tests to preserve context. Run all tests piping the entire output to `.agents/tmp/scratch.txt`, use this for all commands, so that multiple greps can be applied without rerunning the tests.

## Hanging Tests

If instructed, or you suspect, that some tests may hang, use `timeout` to catch these. Run an exponential backoff timeout, starting with two minutes, to catch hanging tests once you find a hanging test, shorten the timeout as much as possible to speed iteration cycles. When debugging hanging tests always pass the `log-runs` flag to detect the tests that dont finish, ie: `timeout 120 cargo test -p beet_core --lib -- --log-runs`. This will help you determine which test didnt complete

## Compile Errors

these sweeping tests perform large compilations and sometimes the mold linker trips up. try running again, and if it still trips up sometimes incrementing the workspace and crates version, ie find-replace Cargo.toml `0.0.9-dev.6`, ..dev.7, will unstuck it. A classic linker error is: `help: you can increase rustc's stack size by setting RUST_MIN_STACK=..` this can usually be fixed by running again, and getting a little further down the compilation step.

## Instructions


Run `just test-core` If you encounter an error, isolate it and run again, ie if the error is in beet_core lib: `cargo test -p beet_core --all-features --lib -- test_name` Fix the error using a subagent, then run the crate again: `cargo test -p beet_core --all-features` When thats clear, run the full suite again `just test-core | tail` After the first complete pass, run fully again, checking for warnings. Fix any warnings encountered

## Success

Success means that `just test-core` passes without warnings, the whole command Upon completion provide a comprehensive summary of what was changed.
