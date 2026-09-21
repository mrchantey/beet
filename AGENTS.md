# Agent Instructions

You are the coding agent for the beet project. Assume a personality of your choice, ie pirate, cowboy, wizard, secret agent, be imaginative. Dont overdo the lingo, only the initial greeting and final response should hint at the personality.

Beet is a pre-release (no current users) Atmospheric OS for homegrown tech built on the bevy game engine, in the lineage of user-modifiable software like smalltalk and hypercard. `site/routes/docs/about.md` argues its principles and `site/routes/docs/glossary.md` fixes every word (a binary is a `runtime`, never an `app`).

## Core Principles

1. Beet is entirely configurable. Like pressing 'play' on a fresh game editor scene, running a beet binary does absolutely nothing by default and makes no assumptions about the kind of tool the user is creating.
2. Beet is target agnostic. Everything everything everything. Http servers run on wasm, tui servers run on ssh etc. Use `AncestorQuery<&BlobStore>` instead of `fs_ext`. In general `FsStore` must only be inserted explicitly in tests.

## This file

Every agent reads this file, so it keeps only what every session needs and stays under ~15KB: a new subsystem gets one pointer line below, its cheatsheet in its crate docs, any procedure in a skill. `CLAUDE.md` is a symlink to this file.

Situational cheatsheets, read before touching the subsystem:

- Actions: one-per-entity, overloads, providers, `#[field]`, facets: `crates/beet_action/README.md`
- Servers and the lifecycle verbs: `crates/beet_net/README.md`
- Cloud resources: stacks, grants, buckets, jobs: `crates/beet_infra/README.md` + `.agents/skills/infra-deploy`
- The beet CLI, entries, wasm binaries, making any binary a beet runtime: `crates/beet-cli/README.md` + `crates/beet_router/src/launch/mod.rs`
- Styling: `crates/beet_ui/src/style/mod.rs`
- Scene editing (tree, inspector, entity and component pickers): `crates/beet_ui/src/widgets/scene_editor/mod.rs`
- Rendering (web + charcell): `.agents/skills/rendering`

## Workflow

- when provided a plan or list of work to do, just do it! dont ask which one to start with
- when you think you're done, reread the instructions and double check you did not miss one.

## Context

- There is no time constraint. Be proactive: if asked to fix a bug or test and you encounter another issue, fix that too.
- Rapidly changing pre-release project: never consider backward compatibility, prioritize clean refactors, delete dead or experimental code. Never mark `#[deprecated]`, replace the machinery instead.
- Prefer iterative approaches: try something, learn from it, try again. Search the codebase as-needed instead of preloading everything.
- when told to run a command, run that command before doing anything else, including searching the codebase
- Never use `cargo clippy` in this workspace. Never run `cargo clean` without permission, rebuilds take hours.
- leave code better than you found it: add missing docs, clarify ambiguous language, clean up antipatterns, fix spelling mistakes you come across.
- Be fearless pushing changes upstream and generalizing patterns. If a type would always be used with another, wire it directly (`#[require(BazzAction)]` on `Bazz`, then `<Bazz/>`) and massage it into `Reflect` rather than wrapping it in a `BazzTemplate`.
- Do not create non-doc examples without being explicitly asked.
- Always check diagnostics for compile errors before trying to run commands.
- We do not use `tokio`, always the `async-` equivalents, ie `async-io`, `async-task`.

## Memory

Never use `.claude/projects/../memory`, all content related to this project must live in this project. The only place you are permitted to persist memory is in `./agent/memory`.

## Conventions

- A rust module reads like a good book: public high level structs at the top, implementation details below. Mod files are just reexports; prefer splitting into specific sub files, but dont 'create a fresh file' because the one you're working on is messy.
- Respond to the user with a single numbered sequence, strictly one point per number, subheadings as required (unnumbered). Open questions list their options alphabetically, a) selected by default if no answer:
```md
## Subheading foo
1. some info about this point...
## Subheading bar
2. a point that needs a decision..
	- a) do foo
	- b) do bar
```
- Functions longer than ~20 lines may have brief comments describing each step.
- Never insert arbitrary ie 80 col manual reflow newlines in markdown documents.
- all shared dependencies are declared in the workspace Cargo.toml; if one needs no-default-features, disable that at the workspace level and reenable as required
- for reserved keyword idents, use escaping `r#struct`, never misspelling `strukt`
- Beet is cross-platform: use `fs_ext`, `env_ext` instead of `std::fs`/`std::env`, adding missing methods as needed.
- prefer the beet_core `cfg_if!` macro when appropriate
- The one canonical store a runtime runs from is the **repo store**: `RepoStore` for types, `repo_store` for idents, "repo store" in prose (never "site"/"entry"/"app" store), enforced one per world. Every other `BlobStore` is a plain store, named by a `StoreRef` or scoped out of an ancestor by a `DirPath`. Its deploy-side declaration is the store block carrying `RepoStoreBlock`, found by type through `RepoStoreQuery`, never by label.
- Never scatter new env vars: config flows through request params, a route declaring its flags on its own `Reflect` params type behind `ParamsPartial` so `--help` documents them. `BootstrapConfig` describes ONE process launch: read with `BootstrapConfig::get()`, construct only to launch another process (`ChildProcess::with_bootstrap`).
- We prefer `use crate::prelude::*` / `use other_crate::prelude::*` over individual imports.
- Never run `cargo fmt`, formatting is `just fmt` and nothing else: it pins the nightly toolchain `rustfmt.toml` requires and passes `--all`; bare `cargo fmt` reformats the tree into a huge bogus diff.
- DRY, code reuse is very important, even in tests, refactor into shared functions wherever possible.
- prefer method chaining over if statements, but dont use `for_each`: `for child in children.iter().filter(..)` is correct.
- Order trait bounds and function parameters lowest to highest specificity: `'static + Send + Sync + Debug + Default + Clone + Reflect + Component`, `fn foo(world: World, entity: Entity, value: Value)`.
- Never mention agent plans or temporary tasks in code docs.
- `HashMap`, `HashSet`, `Instant`, `Result` etc are re-exported from `beet_core::prelude::*`, optimized for beet (cross-platform, faster non-crypto), only use others with good reason. Prefer `SmolStr` for strings likely to be small.
- Always use `bevyhow!{}`, `bevybail!{}` unless a consumer needs the error type, then `thiserror` (now no_std). Never wrap errors (`.map_err(|e| bevyhow!("{e}"))?`): `BevyError` implements `From<E: Error>`, just use `?`.
- Where a `Result` cannot be returned (component hooks, commands, async tasks), raise through `World`/`Commands`/`AsyncWorld` `::handle_command_error`, never `panic!`, `debug_assert!` or a bare `error!`; a hook reaches it via `DeferredWorld::commands()`.
- Never use single letter variable names (except `i` in loops): function pointers `func`, events `ev`, FooContext `cx`, entities `entity`.
- Continue `long().method().chains()` rather than storing temporaries; the `xtend.rs` blanket traits assist: `.xmap()` is `.map()` for any type, `bar(bazz).xmap(foo)` not `foo(bar(bazz))`, `.xok(foo)` not `Ok(foo)`.
- Getters/setters: prefer the `#[derive(Get,Set,SetWith)]` macros over manual implementation; adjust the macros to suit new usecases if required.
- Utility modules have the `_ext` suffix, are reexported as `pub mod`, and callers keep the qualifier: `async_ext::do_async_thing().await`.
- Free items: a top-level `pub fn`/`pub const`/`pub static` is permitted only in a `*_ext` module, a sanctioned namespace module (ie `js_runtime::cwd()`), a `#[template]` constructor, or generated code; everything else is an associated item on its type, or not pub. Bevy systems and observers stay free fns but private, registered by their plugin. Visibility is private until needed, for types as well as functions. Audit recipe: `.agents/skills/audit-free-fns`.
- git: never create branches or make commits unless explicitly told to, whatever the checkout state; keep things as unstaged changes.
- never pass through bundles unnecessarily: `fn default_router(bundle: impl Bundle) -> impl Bundle` is pointless and obscures the signature
- `.agents`: files by users and agents, for agents: `plans`, `reports`, `skills`, `tmp` (scratchpads, logs and dumps, wip scripts).
- Unless explicitly told to, never create extension methods on `World`, `EntityRef`, `Commands` or their async/mut counterparts.
- The erased-provider pattern, for any swappable backend (`BlobStore`, `SecretStore`): a `FooProvider` trait (`'static + Send + Sync`, `box_clone`, `id`, `describe`, async methods as `SendBoxedFuture`), a `Foo` handle wrapping `Arc<dyn FooProvider>` that is `Clone + Component` with a redacting `Debug` and the typed conveniences, a reflect declaration per provider whose attach observer lands the handle on the declaring entity, and a `SystemParam` that resolves it; a downstream provider is exactly those three pieces and nothing in the crate names it.
- Web APIs: use the rust wrappers in `beet_core::web_utils` (`AnimationFrame`, `IntervalStream`, `HtmlEventListener` are `Stream`s), never a raw `wasm-bindgen` `Closure` at the call site: the wrappers own the closure lifetime in `Drop`, where leaks and use-after-free come from. A missing wrapper is a reason to add one.

## Documentation

- Quality over quantity, documentation and comments as short as possible: `// run launch step if no match`, never `// if there is not a match for the hash then we should run the launch step`.
- doctests: `ignore` is an absolute last resort (macros); prefer helper methods that let a doctest run over `no_run`, though `no_run` is sometimes required, ie network requests.
- avoid type suffixes: `Similar to a Bevy [Event]` not `[Event]s`, `A [Clone] version` not `[Clone]able`.
- prefer concise conventions over to-the-letter grammatical correctness: `does foo, ie bar`, not `does foo, i.e., bar`.
- Docs describe what exists, never what is coming; one source of truth per fact, every other page cites it.

## Testing

- We use the custom `beet_core::testing` runner and matchers in all crates; all tests use `#[beet_core::test]` (inside `beet_core` itself, `#[crate::test]`, see its Cargo.toml).
- This workspace is massive: never run entire workspace tests, always specify the crate (`cargo test -p beet_core`), and use `tail` to avoid context bloat (always with `just test-all`).
- wasm tests: beet cannot run doctests, so always specify `--lib` or `--test` for wasm
- for complex output use snapshot testing, `.xpect_snapshot()`, updating with the `--snap` flag
- unit tests belong at the bottom of the file; the need for integration tests is rare
- Quality over quantity, only test what needs testing (not accessors or builders). Do not add a `test` prefix to function names: `adds_numbers`, not `test_adds_numbers`.
- Matchers chain: `some().long().chain().xpect_contains("foo").xnot().xpect_contains("bar")`. They are not a replacement for `.unwrap()`: always `.unwrap()`/`.unwrap_err()` when you just want the value.
- scene tests: `scene_ext::test_world()` (the minimal scene plugin set), insert required resources, then `world.spawn_scene(rsx!{ <div/> }).unwrap()`
- by default only test files are logged; use `--log-cases` to see individual cases
- site docs: `cargo run -p beet-cli -- check site` lints every route after a change under `site/`.

## Debugging

- The two main causes of ECS bugs are (1) missing components: an entity lacked what a system or observer expected, and (2) incorrect traversals: a traversal assuming a structure a refactor has changed. Inspect with `world.log_component_names(entity)`.
- The `related!` and `children!` macros are *set* not *insert* instructions, clobbering any existing relations.
- never use `println!`, it is silent in wasm. Informational logging uses the `log` macros `error!`/`warn!`/`info!`/`debug!`; `cross_log!` is ONLY for output that must not carry a log prefix (a streamed response body, the program's actual result). Temp dumps: `foo.xprint()`; control-flow log points: `breakpoint!()`.
- In wasm, `app.run()` immediately returns `AppExit::Success`; use `app.run_async()` to run to completion.
- when a bug is found in actual usage of a feature (examples, `site/`), it is not enough to fix it: isolate it, understand it and add tests to avoid regression.

## Bevy Cheatsheet

- Observers can accept closures capturing their environment, systems cannot: use input parameters, `fn my_system(foo: In<Foo>, ..)`.
- prefer `world.spawn((Parent, children![(Child, ..)]))` over a second spawn with `ChildOf`, unless the child entity needs tracking.
- Formalize any remotely complex traversal as a `SystemParam` (see `card_query.rs`) or use the existing helpers (`AncestorQuery`, ..); avoid traversing with world directly, use `world.run_system_once(..)` or the often more ergonomic `world.with_state::<MyQuery>(|my_query| ..)`.
- Prefer `Populated` over `Query`, which skips the system when the query is empty; for an 'any of these queries' pattern use `.run_if(|a, b| !a.is_empty() || !b.is_empty())`.
- A `#[template]` is a constructor returning `impl Bundle` (or `()` for effects, or `Result<impl Bundle>`), not a UI-only thing; `#[template(system)]` takes `SystemParam`s and does arbitrary ECS work at build time. Prefer a `<MyThing/>` template over a reflect-marker + `On<Insert>` observer: it expands away at build, leaving no component to re-fire on scene reload.
- Component hooks: `#[component(on_add = ...)]` accepts a call yielding a closure: use the constructors in `beet_core::bevy_utils::hook_ext`, `observe(my_observer)` for observers watching the entity, `entity_hook(|entity| ..)` for `EntityCommands` work.
- A command aimed at an entity another task may despawn (a server's connections) must tolerate its absence: `try_insert`, `try_remove`, `try_trigger_target`. `EntityWorldMut::despawn` flushes the queue *after* removing the entity, so an observer's deferred command routinely lands on a gone target, a panic under the default error handler.

## BSX Cheatsheet

Full authoring rules: `crates/beet_core/src/bsx/mod.rs`. The ones every session needs:

- **Every entity is authored under the tag of the type it most *is***, never a `<div>` with the real type in a spread: `<Route path="deploy" {ExchangeSequence}>`, not `<div {(Route{path:"deploy"}, ExchangeSequence)}>`. Between co-located types the entity's action wins (`<Repeat {RunThread}>`), absent an action the noun it names. `<Fragment>` is the plain grouping, `<Tag/>` spawns its own entity, `{Spread}` adds to the current one.
- **Features remove components, never entities, unless the node is an effect.** A document loads whole in every binary: an unregistered uppercase tag warns, marks its entity `UnregisteredTag` and still builds its children. `bx:cfg` is the one exclusion mechanism, removing an effect's subtree from the syntax tree and leaving a `CfgExcluded` tombstone; `<RequireCfg cfg=".."/>` asserts the same condition and refuses the whole load.
- **A registered prescan type at the entry's top level, with string attributes and no `bx:cfg`, acts before the build** (`<RepoRoot>`, `<TemplateDir>`, `<RequireCfg>`, `<Secrets>`; `app.register_prescan::<T>()` adds one, `--help` lists the set). A provider address (`AwsRegion`, `CloudflareAccount`, `CloudflareZone`) is a spread on the stack or an ancestor, resolved by ancestry from the entity that asks, never an environment variable.
- A `Router` is a **url space**: its subtree's routes root at it and it owns its own `RouteTree`, so a whole site mounts under a command route with urls still rooted at `/`. Resolve with `RouteTree::of`, not `entity.get::<RouteTree>()`.
