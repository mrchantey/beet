# Agent Instructions

You are the coding agent for the beet project.

Beet is a pre-release (no current users) Atmospheric OS for homegrown tech built on the bevy game engine, in the lineage of user-modifiable software like smalltalk and hypercard. `site/routes/docs/about.md` argues its principles and `site/routes/docs/glossary.md` fixes every word (a binary is a `runtime`, never an `app`).

## Core Principles

1. Beet is entirely configurable. Like pressing 'play' on a fresh game editor scene, running a beet binary does absolutely nothing by default and makes no assumptions about the kind of tool the user is creating.
2. Beet is target agnostic. Everything everything everything. Http servers run on wasm, tui servers run on ssh etc. Use `AncestorQuery<&BlobStore>` instead of `fs_ext`, and a `TableStore` for rows, which is declared and never implied by a blob store. In general `FsStore` must only be inserted explicitly in tests.

## This file

Every agent reads this file, so it keeps only what every session needs and stays under ~16KB: a new subsystem gets one pointer line below, its cheatsheet in its crate docs, any procedure in a skill. A skill only this repo runs lives in `.agents/skills`; one a downstream also needs lives in the shared `~/.agents/skills` (`/home/pete/me/arch-config/stow/agents/.agents/skills`), named `beet-*` when it is about beet.

Situational cheatsheets, read before touching the subsystem:

- Actions: one-per-entity, overloads, providers, `#[field]`, facets: `crates/beet_action/README.md`
- Servers and the lifecycle verbs: `crates/beet_net/README.md`
- Cloud resources: stacks, grants, buckets, jobs, a box's ssh authority: `crates/beet_infra/README.md` + `.agents/skills/infra-deploy`
- Deploy credentials, `<stack>/deploy`, a package's `all/deploy` and `--elevated` (no mint verb; an agent relays a refusal to the operator as it is): `crates/beet_infra/README.md`, "Deploy, and deploy elevated"; a repo's own AWS account (`<AwsAccount/>`, `beet aws --account`) and a person's way in (`<IdentityCenter/>`, `beet sso login`): "The AWS account"
- AWS calls (`AwsClient`, signed, every target, no SDK or cli): `crates/beet_net/src/client/aws/mod.rs`; asking the operator: `OperatorPrompt`
- The beet CLI, entries, wasm binaries, making any binary a beet runtime: `crates/beet-cli/README.md` + `crates/beet_router/src/launch/mod.rs`
- Styling: `crates/beet_ui/src/style/mod.rs`
- Scene editing (tree, inspector, entity and component pickers): `crates/beet_ui/src/widgets/scene_editor/mod.rs`
- Rendering (web + charcell): the `beet-rendering` skill
- Tables (`TableStore`, the naive `JsonTableStore`, rows, preconditions, partitions, record tables, the capability matrix, the wire, Durable Objects): `crates/beet_net/README.md`, "Tables"
- Derived files (`Derivation`, `<Derive/>`, `beet check`'s stale or missing one): `crates/beet_net/src/store/derivation.rs` + `Provenance`'s module docs
- Secrets (the age identity, `secrets.toml`, the `vault`/`secrets` verbs, the `SecretStore` seam every secret a process reads or a deploy mints goes through): `crates/beet_core/src/secrets/mod.rs` + `site/routes/docs/secrets.md`
- Atproto (records and their primitives, an account's PDS as a `Pds` scoped to one repo, the converge, `<AtprotoAccount/>`, the live test account): `crates/beet_core/src/atproto/mod.rs` + `crates/beet_net/src/atproto/mod.rs`; the standard.site records: `crates/beet_router/src/router/site/syndication/standard_site/mod.rs`
- The compiler (a branch into scene, branch and package records, `SceneMeta`, `PackageMeta`, `Def`, `beet compile`): `crates/beet_scene/src/compile/mod.rs` + `crates/beet_core/src/package/mod.rs`
- Lexicons (a tree rendered as, or parsed from, one atproto lexicon object by NSID, beside the media type registries; `UnknownLexicon`): `crates/beet_ui/src/lexicon/mod.rs`

## Workflow

- when provided a plan or list of work to do, just do it! dont ask which one to start with
- when you think you're done, reread the instructions and double check you did not miss one.

## Conventions

- Order trait bounds and function parameters lowest to highest specificity: `'static + Send + Sync + Debug + Default + Clone + Reflect + Component`, `fn foo(world: World, entity: Entity, value: Value)`.
- prefer method chaining over if statements, but dont use `.for_each(..)`: `for child in children.iter().filter(..)` is correct.
- Prefer `use crate::prelude::*` / `use other_crate::prelude::*` over individual imports.
- Beet is cross-platform: use `fs_ext`, `env_ext` instead of `std::fs`/`std::env`, adding missing methods as needed.
- prefer `beet::prelude::cfg_if!` over attributes where appropriate
- The one canonical store a runtime runs from is the **repo store**: `RepoStore` for types, `repo_store` for idents, "repo store" in prose (never "site"/"entry"/"app" store), enforced one per world. Every other `BlobStore` is a plain store, named by a `StoreRef`, or one field per role where a consumer needs several, or scoped out of an ancestor by a `DirPath`. Its deploy-side declaration is the store block carrying `RepoStoreBlock`, found by type through `RepoStoreQuery`, never by label.
- Never scatter new env vars: config flows through request params, and an extractor declares a param by taking it, so `--help` documents it: a route or an `#[action]` middleware in its signature (`#[param] fix: bool`, `entry: Params<EntryParams>`), a hand written one with `app.install(SubtreeParams::declare_on::<Server, T>())`. Every `CliServer` is strict (a lenient command takes `any: AnyParams`); named groups and the launch only knobs no route takes are `crates/beet_router/README.md`, Params. `BootstrapConfig` describes ONE process launch: read with `BootstrapConfig::get()`, construct only to launch another process (`ChildProcess::with_bootstrap`).
- Never run `cargo fmt`: `just fmt` pins the nightly `rustfmt.toml` needs and passes `--all`; bare `cargo fmt` makes a huge bogus diff.
- The format is chosen at the edge: code holds serde types and `MediaType`'s serde or a format registry picks the bytes, never a `json_ext::`/`postcard::`/`toml::` call unless the format is the protocol (an http api's json, atproto's dag-cbor, a file whose name fixes it). JSON is beet's own, exact for every float: `json_ext` for a serde type, `Value::parse_json`/`to_json_string` for a value, `value!` for a literal; `serde_json` is only the reference beet_core's conformance tests compare against.
- `HashMap`, `HashSet`, `Instant`, `Result` etc are re-exported from `beet_core::prelude::*`, optimized for beet (cross-platform, faster non-crypto), only use others with good reason. Prefer `SmolStr` for strings likely to be small.
- Always use `bevyhow!{}`, `bevybail!{}` unless a consumer needs the error type, then `thiserror` (now no_std). Never wrap errors (`.map_err(|e| bevyhow!("{e}"))?`): `BevyError` implements `From<E: Error>`, just use `?`.
- Where a `Result` cannot be returned (component hooks, commands, async tasks), raise through `World`/`Commands`/`AsyncWorld` `::handle_command_error`, never `panic!`, `debug_assert!` or a bare `error!`; a hook reaches it via `DeferredWorld::commands()`.
- Continue `long().method().chains()` rather than storing temporaries; the `xtend.rs` blanket traits assist: `.xmap()` is `.map()` for any type, `bar(bazz).xmap(foo)` not `foo(bar(bazz))`, `.xok(foo)` not `Ok(foo)`.
- Getters/setters: `#[derive(Get,Set,SetWith)]`, never by hand, adjusting the macros for a new use case. A validated string newtype over `SmolStr` derives `StringPrimitive`, its `parse` the one way in.
- Utility modules have the `_ext` suffix, are reexported as `pub mod`, and callers keep the qualifier: `async_ext::do_async_thing().await`.
- Free items: a top-level `pub fn`/`pub const`/`pub static` is permitted only in a `*_ext` module, a sanctioned namespace module (ie `js_runtime::cwd()`), a `#[template]` constructor, or generated code; everything else is an associated item on its type, or not pub. Bevy systems and observers stay free fns but private, registered by their plugin. Visibility is private until needed, for types as well as functions. Audit recipe: the `beet-audit-free-fns` skill.
- git: never create branches or make commits unless explicitly told to, whatever the checkout state; keep things as unstaged changes.
- never pass through bundles unnecessarily: `fn default_router(bundle: impl Bundle) -> impl Bundle` is pointless and obscures the signature
- Unless explicitly told to, never create extension methods on `App`, `World`, `EntityRef`, `Commands` or their async/mut counterparts. A registration is an installer (`crates/beet_core/src/bevy_utils/install.rs`), the type or value deciding what it registers: a `TypeInstaller` installed by type, `app.install_types::<(Route, Redirect)>()` (a template, a prescan), or an `Installer` installed by value, `app.install(LeafletRenderer)` (a doc renderer, a constructor returning `impl Installer`); a bevy `Plugin` is both, installed only when absent; tuples install in order.
- An `#[action]` route spells its path `#[action(route = "..")]`, never a hand `#[require(PathPartial = ..)]`.
- The erased-provider pattern, for any swappable backend (`BlobStore` over `BlobProvider`, `TableStore` over `TableProvider`, `SecretStore` over `SecretStoreProvider`): a provider trait (`'static + Send + Sync`, `box_clone`, `id`, `describe`, async methods as `SendBoxedFuture`), a handle wrapping the trait object in an `Arc` that is `Clone + Component` with a redacting `Debug` and the typed conveniences, a reflect declaration per provider whose attach observer lands the handle on the declaring entity, and a `SystemParam` that resolves it; a downstream provider is exactly those three pieces and nothing in the crate names it.
- Web APIs: use the rust wrappers in `beet_core::web_utils` (`AnimationFrame`, `IntervalStream`, `HtmlEventListener` are `Stream`s), never a raw `wasm-bindgen` `Closure` at the call site, since a wrapper owns its lifetime in `Drop`; a missing wrapper is a reason to add one.


## Testing

- We use the custom `beet_core::testing` runner and matchers in all crates; all tests use `#[beet_core::test]` (inside `beet_core` itself, `#[crate::test]`, see its Cargo.toml).
- This workspace is massive: never run entire workspace tests, always specify the crate (`cargo test -p beet_core`), and use `tail` to avoid context bloat (always with `just test-all`).
- wasm tests: beet cannot run doctests, so always specify `--lib` or `--test` for wasm
- for complex output use snapshot testing, `.xpect_snapshot()`, updating with the `--snap` flag
- unit tests belong at the bottom of the file; the need for integration tests is rare
- Quality over quantity, only test what needs testing (not accessors or builders). No `test` prefix: `adds_numbers`, not `test_adds_numbers`.
- Matchers chain: `some().long().chain().xpect_contains("foo").xnot().xpect_contains("bar")`. They never replace `.unwrap()`/`.unwrap_err()` when you just want the value.
- scene tests: `scene_ext::test_world()` (the minimal scene plugin set), insert required resources, then `world.spawn_scene(rsx!{ <div/> }).unwrap()`
- by default only test files are logged; use `--log-cases` to see individual cases
- site docs: `cargo run -p beet-cli -- check site` lints every route after a change under `site/`.

## Debugging

- The two main causes of ECS bugs are (1) missing components: an entity lacked what a system or observer expected, and (2) incorrect traversals: a traversal assuming a structure a refactor has changed. Inspect with `world.log_component_names(entity)`.
- The `related!` and `children!` macros are *set* not *insert* instructions, clobbering any existing relations.
- **stdout is the program's answer, stderr is everything else.** Never `println!`, it is silent in wasm. Logging uses `error!`/`warn!`/`info!`/`debug!`, on stderr; `cross_log!` writes stdout and is ONLY for the answer itself (a streamed response body, a rendered result), so a one-shot pipes clean. Temp dumps: `foo.xprint()`; control-flow log points: `breakpoint!()`.
- In wasm, `app.run()` immediately returns `AppExit::Success`; use `app.run_async()` to run to completion.
- a bug found in real use (examples, `site/`) is never just fixed: isolate it, understand it and add a regression test.

## Bevy Cheatsheet

- Observers can accept closures capturing their environment, systems cannot: use input parameters, `fn my_system(foo: In<Foo>, ..)`.
- prefer `world.spawn((Parent, children![(Child, ..)]))` over a second spawn with `ChildOf`, unless the child entity needs tracking.
- Formalize any remotely complex traversal as a `SystemParam` (see `card_query.rs`) or use the existing helpers (`AncestorQuery`, `RenderTreeQuery` for any walk over a rendered tree, which steps through each `Portal`, ..); never traverse with world directly: `world.with_state::<MyQuery>(|my_query| ..)` or `world.run_system_once(..)`.
- Prefer `Populated` over `Query`, which skips the system when the query is empty; for an 'any of these queries' pattern use `.run_if(|a, b| !a.is_empty() || !b.is_empty())`.
- A `#[template]` is a constructor returning `impl Bundle` (or `()` for effects, or `Result<impl Bundle>`), not a UI-only thing; `#[template(system)]` takes `SystemParam`s and does arbitrary ECS work at build time. Prefer a `<MyThing/>` template over a reflect-marker + `On<Insert<Marker>>` observer: it expands away at build, leaving no component to re-fire on scene reload.
- Component hooks: `#[component(on_add = ...)]` accepts a call yielding a closure: use the constructors in `beet_core::bevy_utils::hook_ext`, `observe(my_observer)` for observers watching the entity, `entity_hook(|entity| ..)` for `EntityCommands` work.
- A command aimed at an entity another task may despawn (a server's connections) must tolerate its absence: `try_insert`, `try_remove`, `try_trigger_target`. Since `EntityWorldMut::despawn` flushes the queue after removing the entity, an observer's deferred command often lands on a gone target and panics.

## BSX Cheatsheet

Full authoring rules: `crates/beet_scene/src/formats/bsx/mod.rs`. The ones every session needs:

- **Every entity is authored under the tag of the type it most *is***, never a `<div>` with the real type in a spread: `<Route path="deploy" {ExchangeSequence}>`, not `<div {(Route{path:"deploy"}, ExchangeSequence)}>`. Between co-located types the entity's action wins (`<Repeat {RunThread}>`), absent an action the noun it names. `<Fragment>` is the plain grouping, `<Tag/>` spawns its own entity, `{Spread}` adds to the current one. A bare attribute or tag means what HTML means, `bx:` marks beet's: a prefab is `<template>`, a slot `<slot>`, a script beet runs `<script bx:run>`.
- **A `bx:` directive is sugar for explicit BSX**, rewritten as a document lowers to a scene body, with no world (`bx:onclick` is a child `<script bx:event="click">`); a `#[template]` needs one, run at spawn. `app.install(MyDirective)` adds one; an unmatched `bx:` key refuses the document.
- **Features remove components, never entities, unless the node is an effect.** A document loads whole in every binary: an unregistered uppercase tag or spread is held with its value in the entity's `Unregistered` and a tag still builds its children. `bx:cfg` is the one exclusion mechanism, pruning an effect's subtree as its body spawns and leaving a `CfgExcluded` tombstone; `<RequireCfg cfg=".."/>` asserts the same condition and refuses the whole load.
- **A registered prescan type in an entry, with string attributes and no `bx:cfg`, acts before the build**; a `<Sequence>` scene, whose every child is a step, declares `<RequireCfg/>` beside it (`<RepoRoot>`, `<RequireCfg>`, `<Secrets>`, `--help` lists the set). A provider address (`AwsRegion`, `CloudflareAccount`, `CloudflareZone`) is a spread on the stack or an ancestor, resolved by ancestry from the entity that asks, never an environment variable.
- **Another document is an `Instance`, by path**: `<Instance document="widgets/Card.bsx" props={{title:"Hi"}}/>`, `./` from its document, else from the entry's directory.
- A `Router` is a **url space**: its subtree's routes root at it and it owns its own `RouteTree`, so a whole site mounts under a command route with urls still rooted at `/`. Resolve with `RouteTree::of`, not `entity.get::<RouteTree>()`.
