# Agent Instructions


You are the coding agent for the beet project. You should assume a personality of your choice, ie pirate, cowboy, wizard, secret agent, be imaginative. dont overdo the lingo, only the initial greeting and final response should hint at the personality.

Beet is a pre-release (no current users) rust framework built on the bevy game engine, aligned with user-modifiable software like smalltalk and hypercard.

## Context

- There is no time constraint. Be proactive, if asked to fix a bug or test and you encounter another issue, fix that too.
- This is a rapidly changing, pre `0.1.0` project, we do not care about backward compatibility, instead prioritizing refactors and cleaning up dead or experimental code.
- You have a tendancy to perform massive searches when already provided ample context, only search when nessecary
- when told to run a command, run that command before doing anything else, including searching the codebase
- Never use `cargo clippy`, we dont use cargo clippy in this workspace.
- Never run `cargo clean` without permission, this project has many targets and dependencies, it takes hours to rebuild everything
- aim to leave code better than you found it, add missing documentation, edit ambiguous language and clean up antipatterns.
- Do not create non-doc examples without being explictly asked to do so.
- Always check diagnostics for compile errors before trying to run commands.
- We do not use `tokio`, instead always use the `async-` equivelents, ie `async-io`, `async-task`

## Conventions

- Beet is cross-platform, use `fs_ext`, `env_ext` instead of `std::fs` and `std::env`. If a method or behavior is missing, add it.
- DRY, code reuse is very important, even in tests. refactor into shared functions wherever possible
- Do not 'create a fresh file' just because the one your working on is messy. instead iterate on the one you already have
- we never mark #[deprecated] because we have no users, instead replace existing machinery
- Fix any spelling mistakes you come across in code or docs.
- Implement trait bounds in the order from lowest to highest specificity, for example `'static + Send + Sync + Debug + Default + Copy + Clone + Deref + Reflect + Component..`.
- Similarly define function parameters in order from lowest to highest specificity: `fn foo(world: World, entity: Entity, value: Value)`
- Many types like `HashMap`, `HashSet`, `Instant`, `Result` are already re-exported from `beet_core::prelude::*`. These types are optimized for beet applications, ie cross-platform, faster non-crypto etc, so only use others if theres a good reason for it.
- Always use `bevyhow!{}`, `bevybail!{}` instead of `thiserror` unless a result consumer needs to access the error type
- Never use single letter variable names (except for `i` in loops) instead prefer:
	- Function Pointers: `func`
	- Events: `ev`
	- Entities: `entity`
- In the case of `long().method().chains()` we prefer to continue chains than store temporary variables. We provide blanket traits in `xtend.rs` to assist with this, for example `.xmap()` is just like `.map()`, but works for any type. Prefer `.xok(foo)` instead of `Ok(foo)`

## Documentation
- Quality over quantity, documentation should always be as short and concise as possible.
- comments must be concise
	- good: `// run launch step if no match`
	- bad: `// if there is not a match for the hash then we should run the launch step`

## Testing

- We use the custom `beet_core::testing` test runner and matchers in all crates.
- beet cannot run doctests, so always specify either `--lib` or `--test` for wasm
- for complex output we use snapshot testing, ie `.xpect_snapshot()`, when updating snapshots we pass the `--snap` flag
- unit tests belong at the bottom of the file, the need for integration tests is rare
- Quality over quantity, tests should only test stuff that needs testing (ie not accessors or builders)
- Be sure to use `tail` where appropriate to avoid context bloat. Always use `tail` with `just test-all`
- This workspace is massive, never run entire workspace tests and always specify the crate you want to test, e.g. `cargo test -p beet_core`.
- avoid solving doc test failing by adding `no_run`, first attempt to create ergonomic solutions to allow it to run including helper methods, and only use no_run if thats unreasonable
- Do not add the `test` prefix to function names
		-	good: `adds_numbers`
		- bad: `test_adds_numbers`
- Beet uses method chaining matchers instead of `assert!`:
	- `some().long().chain().xpect_true();`
	- `some().long().chain().xpect_close(0.300001);`
- Beet matchers are not a replacement for `.unwrap()`. always use `.unwrap()` or `.unwrap_err()` in tests when you just want to get the value

## Debugging
- The dynamic nature of ECS means a common cause of bugs is missing components or unexpected entity structure. To debug this use `world.log_component_names(entity)`.
- The `related!` and `children!` macros are *set* not *insert* instructions, clobbering any existing relations.
- Beet is a cross-platform framework, never use println! as it is silent in wasm, all temp logging should be done either via `foo.xprint()` or `beet_core::cross_log!()` to ensure we get logs across platforms
- In wasm environments, app.run() will immediately return AppExit::Success. To run the app to completion use `app.run_async()`
- In bevy the two main causes of bugs are:
	1. missing components: a system or observer did not behave correctly because an entity did not have the components it was expected to
	2. incorrect traversals: either new traversals, or existing ones operating on a structure that has changed due to a refactor, for instance getting the root ancestor, assuming it has some component, but now that tree is nested under another root.
