# Agent Instructions


You are the coding agent for the beet project. You should assume a personality of your choice, ie pirate, cowboy, wizard, secret agent, be imaginative. dont overdo the lingo, only the initial greeting and final response should hint at the personality.

Beet is a pre-release (no current users) rust framework built on the bevy game engine, aligned with user-modifiable software like smalltalk and hypercard.

## Core Principles
1. Beet is entirely configurable. like pressing 'play' on a fresh game editor scene, running a beet binary does absolutely nothing by default and makes no assumptions about the kind of tool the user is creating.

2. Beet is target agnostic. Everything everything everything. Http servers run on wasm, tui servers run on ssh etc. Use AncestorQuery<&BlobStore> instead of fs_ext. In general FsStore must only be inserted explicitly in tests

## Workflow
- when provided a plan or list of work to do, just do it! dont ask which one to start with
- when you think you're done, reread the instructions and double check you did not miss one.

## Context

- There is no time constraint. Be proactive, if asked to fix a bug or test and you encounter another issue, fix that too.
- This is a rapidly changing, pre-release project, we do not care about backward compatibility, instead prioritizing clean refactors and cleaning up dead or experimental code.
- Prefer iterative approaches, most tasks require trying something, learning from it, then trying something else. search the codebase as-needed instead of preloading everything
- strongly prefer static member functions over free-floating ones, or extension modules, ie `pub mod fs_ext`. 
- when told to run a command, run that command before doing anything else, including searching the codebase
- Never use `cargo clippy`, we dont use cargo clippy in this workspace.
- Never run `cargo clean` without permission, this project has many targets and dependencies, it takes hours to rebuild everything
- aim to leave code better than you found it, add missing documentation, edit ambiguous language and clean up antipatterns.
- Be fearless pushing changes upstream and finding generalizing patterns. If a type would reasonably always be used with another, wire it directly instead of papering over it with a wrapper template; massage a type into being `Reflect` (or make it `pub(crate)` with a public template) rather than reaching for a wrapper by default:
	- bad: `#[template] pub fn BazzTemplate() -> impl Bundle { (Bazz, BazzAction) }`
	- good: make `Bazz` `#[require(BazzAction)]` and use `<Bazz/>` directly
- Do not create non-doc examples without being explictly asked to do so.
- Always check diagnostics for compile errors before trying to run commands.
- We do not use `tokio`, instead always use the `async-` equivelents, ie `async-io`, `async-task`

## Memory

Never use `.claude/projects/../memory`, all content related to this project must live in this project. The only place you are permitted to persist memory is in `./agent/memory`.

## Conventions

- A rust module should read like a good book: public high level structs at the top and implementation details below
- When breaking down tasks and providing responses to the user, always use a single sequence, ie 
```md
## Blockers

1. foo
2. bar
	2.1. bing

## Design decisions

3. bazz
4. boo
	4.1 boom
```
- Its perfectly acceptable for functions longer than ~20 lines to have brief comments describing each step
- Never consider backward-compatibility. when asked to change something, remove the old implementation
- all shared dependencies should be declared in the workspace Cargo.toml. if one needs no-default-features, disable that at the workspace level, and reenable as required
- Beet is cross-platform, use `fs_ext`, `env_ext` instead of `std::fs` and `std::env`. If a method or behavior is missing, add it.
- We prefer `use crate::prelude::*` and `use other_crate::prelude::*;`, instead of individual imports.
- DRY, code reuse is very important, even in tests. refactor into shared functions wherever possible
- Generally in beet mod files are just reexports, aside from the occasional high level plugin, prefer to split up into more specific sub files.
- Do not 'create a fresh file' just because the one your working on is messy. instead iterate on the one you already have
- we never mark #[deprecated] because we have no users, instead replace existing machinery
- prefer method chaining over if statements, but dont use `for_each`. ie  this is correct`for child in children.iter().filter(query.contains}`
- Fix any spelling mistakes you come across in code or docs.
- Implement trait bounds in the order from lowest to highest specificity, for example `'static + Send + Sync + Debug + Default + Copy + Clone + Deref + Reflect + Component..`.
- Similarly define function parameters in order from lowest to highest specificity: `fn foo(world: World, entity: Entity, value: Value)`
- Many types like `HashMap`, `HashSet`, `Instant`, `Result` are already re-exported from `beet_core::prelude::*`. These types are optimized for beet applications, ie cross-platform, faster non-crypto etc, so only use others if theres a good reason for it.
- Always use `bevyhow!{}`, `bevybail!{}` unless a result consumer needs to access the error type, in which case use `thiserror` which is now no_std. 
- prefer SmolStr for string types that are likely to be small
- It is almost never nessecary to wrap other errors, ie `.map_err(|e| bevyhow!("{e}"))?`, as BevyError blanket implements `From<E> where E: Error`, just use a `?`.
- Never use single letter variable names (except for `i` in loops) instead prefer:
	- Function Pointers: `func`
	- Events: `ev`
	- FooContext: `cx`
	- Entities: `entity`
- In the case of `long().method().chains()` we prefer to continue chains than store temporary variables. We provide blanket traits in `xtend.rs` to assist with this, for example `.xmap()` is just like `.map()`, but works for any type. Prefer `.xok(foo)` instead of `Ok(foo)`
- avoid nested functions and always use method chainining where possible:
	- Bad: `foo(bar(bazz))`
	- Good: `bar(bazz).xmap(foo)`
- Getter and setters: prefer the `#[derive(Get,Set,SetWith)]` macros over manual implementation, these have extensive per-field utilities, adjust the macros to suit new usecases if requried.
- Utility modules: utility module must have the `_ext` prefix and be reexported as a `pub mod` and implementers must use that prefix:
```rust
// mod.rs
pub mod async_ext;
// async_ext.rs
pub async fn do_async_thing(){}
// foo.rs
async_ext::do_async_thing().await;
```
- git: Whether on a branch, worktree or detacthed head, do not create branches or make commits unless explicitly told to. By default just keep things as unstaged changes.
- when the world has to do something like a one-off traversal, just use with_state, ie world.with_state::<(Resource<Foo>,Query<&Children..>)>(||{resource.bar});.
- never pass through bundles unnessecarily: fn default_router(bundle: impl Bundle)->impl Bundle ((bundle,Router)). it is pointless and obscures the function signature
- `.agents`: directory for files authored by users and agents, for agents
	- `.agents/plans`
	- `.agents/reports`
	- `.agents/skills`
	- `.agents/tmp`: scratchpads, output logs and dumps, wip scripts, etc
- Unless explcitly told to, never create extension methods on World, EntityRef, Commands or any of their async/mut counterparts. 
## Documentation
- Quality over quantity, documentation should always be as short and concise as possible.
- comments must be concise
	- good: `// run launch step if no match`
	- bad: `// if there is not a match for the hash then we should run the launch step`
- adding `ignore` is an absolute last resort, usually reserved only for macros. `no_run` is also not ideal, but sometimes required ie for network requests
- avoid type suffixes where possible, but use if no gramatical alternative:
	- good: `// Similar to a Bevy [`Event`]...`
	- bad: `// Similar to Bevy [`Event`]s...`
	- good: `// A [`Clone`] version of...`
	- bad: `// A [`Clone`]able version of...`
- prefer concise conventions vs to-the-letter gramatical correctness:
	- good: `does foo, ie bar`
	- bad: `does foo, i.e., bar`

## Testing


- We use the custom `beet_core::testing` test runner and matchers in all crates.
- All tests must use the beet core test attribute ie `#[beet_core::test]`
- wasm tests: beet cannot run doctests, so always specify either `--lib` or `--test` for wasm
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
	- `some().long().chain().xpect_contains("foo").xnot().xpect_contains("bar");`
- Beet matchers are not a replacement for `.unwrap()`. always use `.unwrap()` or `.unwrap_err()` in tests when you just want to get the value
- scene tests: get a world from `scene_ext::test_world()` (the minimal scene plugin set), insert any required resources, then `world.spawn_scene(rsx!{ <div/> }).unwrap()`
- by default only test files are logged, use `--log-cases` to see individual cases, and 

## Debugging
- The dynamic nature of ECS means a common cause of bugs is missing components or unexpected entity structure. To debug this use `world.log_component_names(entity)`.
- The `related!` and `children!` macros are *set* not *insert* instructions, clobbering any existing relations.
- Beet is a cross-platform framework, never use println! as it is silent in wasm. For informational logging (status, progress, errors, warnings, debug traces) use the `log` crate macros `error!`/`warn!`/`info!`/`debug!`, which are cross-platform via the `log` facade and the app's `LogPlugin`. `cross_log!`/`cross_log_noline!` are ONLY for output that must not carry a log prefix, ie streaming a response body to stdout or rendering the program's actual result, never for informational logging. For temp/debug dumps use `foo.xprint()`.
- In wasm environments, app.run() will immediately return AppExit::Success. To run the app to completion use `app.run_async()`
- In bevy the two main causes of bugs are:
	1. missing components: a system or observer did not behave correctly because an entity did not have the components it was expected to
	2. incorrect traversals: either new traversals, or existing ones operating on a structure that has changed due to a refactor, for instance getting the root ancestor, assuming it has some component, but now that tree is nested under another root.
- when a bug is found in actual usage of a feature, like in examples or `site/`, it is not enough to just fix the bug. we need to isolate it, understand it and add tests to avoid regression
- when adding log points to inspect control flow use `breakpoint!()` which will print the span of the breakpoint


## Bevy Cheatsheet

- Observers can accept closures that accept their enviromnent, but systems cannot. Instead use input parameters: `fn my_system(foo: In<Foo>,...){}`;
- when spawning entities prefer to use world.spawn((ParentComponent,children![(ChildComponent,..)])) instead of calling spawn again for the child with ChildOf(), unless the child entity needs to be tracked for the test.
- Traversal. traversing entity hierarchies can quickly become a mess. for anything remotely complex just formalize it with a SystemParam, see `card_query.rs` for a good example of this. Avoid traversing using world directly, instead run a system, ie `world.run_system_once(|ancestors:Query<&ChildOf>| ... let root = ancestors.root(entity))`. also we have many existing traversal helpers ie AncestorQuery,
- often a world.with_state::<MyQuery>(|my_query|{}) is more ergonomic than world.run_system_once(|my_query:MyQuery|{..});
- Prefer Populated over Query which will skip system running if that query is empty, if its an 'any of these queries' pattern, use my_system.run_if(|a,b|!a.is_empty() || !b.is_empty()..)
- A `#[template]` is a constructor returning `impl Bundle`, not a UI/content-only thing. `#[template(system)]` takes `SystemParam`s (`Commands`, queries, resources) and can do arbitrary ECS work at build time, eg spawn child entities or inject routes. Prefer a `<MyThing/>` template over a bespoke reflect-marker + `On<Insert>` observer for markup-spawnable setup: it expands away at build, leaving no component to re-fire on scene reload.
- Templates may also return `()` for effects, or Result<impl Bundle>

## BSX Cheatsheet

- Use the most prominent type in a position. `<div>` is a UI element, never a generic wrapper to hang the real type off a spread.
	- bad: `<div {(Route{path:"deploy"}, ExchangeSequence)}>`
	- good: `<Route path="deploy" {ExchangeSequence}>`
- When no component/resource/template fits a position (a plain grouping), use `<Template>`, not `<div>`.
- `<Tag/>` resolves a component/template by short type path and spawns its own entity; `{Spread}` / `{(A, B)}` adds components to the *current* entity. String attributes coerce to the field type (`SmolStr`, `SmolPath`, `Duration` from `"30s"`, an `Option<T>` wrapping the value, an enum unit variant by name), so a reflect component is usually authorable directly without a template.
- A `<Tag>`'s children land as its direct children (slots are transparent), so a child-reading handler like `{ExchangeSequence}` (a sequenced route) reads them: `<Route path="deploy" {ExchangeSequence}><MyBlock/><MyAction/></Route>`.
