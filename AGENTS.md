# Agent Instructions

## Context

- when discussing code, assume the language is rust unless user specifies otherwise
- You have a tendancy to perform massive searches when already provided ample context, only search when nessecary
- when told to run a command, run that command before doing anything else, including searching the codebase
- Always use rust conventions, unit tests at the bottom of the file are preferred over separate test files.
- Never use `cargo clippy`, we dont use cargo clippy in this workspace.
- Never run `cargo clean` without permission, this project has many targets and dependencies, it takes hours to rebuild everything
- aim to leave code better than you found it, add missing documentation, edit ambiguous language and clean up antipatterns.
- Do not create non-doc examples without being explictly asked to do so.
- Always check diagnostics for compile errors before trying to run commands.
- We do not use `tokio`, instead always use the `async-` equivelents, ie `async-io`, `async-task`

## Style

- Code reuse is very important, even in tests. refactor into shared functions where possible
- Always greet the user by saying something foolish, here are some examples but you should come up with your own instead of using these directly:
	- jumbajumba
	- chickadoodoo
	- i'm a little teapot
	- choo choo i'm a tank engine
	- whats good chicken

- Implement trait bounds in the order from lowest to highest specificity, for example `'static + Send + Sync + Debug + Default + Copy + Clone + Deref + Reflect + Component..`.
- Never use single letter variable names, except for `i` in loops, instead prefer:
	- Function Pointers: `func`
	- Events: `ev`
	- Entities: `ent`
- Do not 'create a fresh file' just because the one your working on is messy. instead iterate on the one you already have
- In the case of `long().method().chains()` we prefer to continue chains than store temporary variables. We provide blanket traits in `xtend.rs` to assist with this, for example `.xmap()` is just like `.map()`, but works for any type.


## Documentation
- documentation should always be as short and concise as possible.
- comments must be consice
	- good: `// run launch step if no match`
	- bad: `// if there is not a match for the hash then we should run the launch step`

## Testing
- This workspace is massive, never run entire workspace tests and always specify the crate you want to test, e.g. `cargo test -p beet_core`.
- We use the custom `sweet` test runner and matchers in all crates.
- Do not add the `test` prefix to function names
		-	good: `adds_numbers`
		- bad: `test_adds_numbers`
- Sweet uses method chaining matchers instead of `assert!`:
	- `some().long().chain().xpect_true();`
	- `some().long().chain().xpect_close(0.300001);`
- Sweet matchers are not a replacement for `.unwrap()`. always use `.unwrap()` or `.unwrap_err()` in tests when you just want to get the value
