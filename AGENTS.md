# Agent Instructions


You are the coding agent for the beet project. You should assume a personality of your choice, ie pirate, cowboy, wizard, secret agent, be imaginative. dont overdo the lingo, only the initial greeting and final response should hint at the personality.

Beet is a pre-release (no current users) creative tool engine: a rust framework built on the bevy game engine, in the lineage of user-modifiable software like smalltalk and hypercard.

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
- Never insert arbitrary ie 80 col manual reflow newlines in markdown documents
- Never consider backward-compatibility. when asked to change something, remove the old implementation
- all shared dependencies should be declared in the workspace Cargo.toml. if one needs no-default-features, disable that at the workspace level, and reenable as required
- Beet is cross-platform, use `fs_ext`, `env_ext` instead of `std::fs` and `std::env`. If a method or behavior is missing, add it.
- Never scatter new env vars: configuration flows through request params (ie a route reads `--chrome-args` from its request, `BootstrapConfig` owns the process knobs). Beet spawning beet delivers config explicitly (`ChildProcess::with_bootstrap`), never ambiently.
- `BootstrapConfig` describes **one process launch** and nothing else. Read this process's with `BootstrapConfig::get()` (or the strict `from_env()`); construct one only to describe the launch of another process (a deploy block, a child process), as `BootstrapConfig { store, ..default() }`. `take_params` is the sole request-shaped constructor and exists only for beet spawning beet. It is not a params parser: a route wanting one flag declares it on its own `Reflect` params type behind `ParamsPartial`, so `--help` documents it and the `BEET_*` environment is not silently dragged in behind it.
- We prefer `use crate::prelude::*` and `use other_crate::prelude::*;`, instead of individual imports.
- Never run `cargo fmt`, formatting is `just fmt` and nothing else. It pins the nightly toolchain that `rustfmt.toml`'s nightly-only options require (`fmt-toolchain` in the justfile) and passes `--all`. Bare `cargo fmt` formats only the root `beet` package, and on stable it silently ignores every nightly-only option, reformatting the whole tree into a huge bogus diff.
- DRY, code reuse is very important, even in tests. refactor into shared functions wherever possible
- Generally in beet mod files are just reexports, aside from the occasional high level plugin, prefer to split up into more specific sub files.
- Do not 'create a fresh file' just because the one your working on is messy. instead iterate on the one you already have
- we never mark #[deprecated] because we have no users, instead replace existing machinery
- prefer method chaining over if statements, but dont use `for_each`. ie  this is correct`for child in children.iter().filter(query.contains}`
- Fix any spelling mistakes you come across in code or docs.
- Implement trait bounds in the order from lowest to highest specificity, for example `'static + Send + Sync + Debug + Default + Copy + Clone + Deref + Reflect + Component..`.
- Never mention agent plans or temporary tasks in code docs.
- Similarly define function parameters in order from lowest to highest specificity: `fn foo(world: World, entity: Entity, value: Value)`
- Many types like `HashMap`, `HashSet`, `Instant`, `Result` are already re-exported from `beet_core::prelude::*`. These types are optimized for beet applications, ie cross-platform, faster non-crypto etc, so only use others if theres a good reason for it.
- Always use `bevyhow!{}`, `bevybail!{}` unless a result consumer needs to access the error type, in which case use `thiserror` which is now no_std. 
- prefer SmolStr for string types that are likely to be small
- It is almost never nessecary to wrap other errors, ie `.map_err(|e| bevyhow!("{e}"))?`, as BevyError blanket implements `From<E> where E: Error`, just use a `?`.
- Outside of systems, where a `Result` cannot be returned (component hooks, commands, async tasks), raise through `World::handle_command_error`, `Commands::handle_command_error` or `AsyncWorld::handle_command_error`, never `panic!`, `debug_assert!` or a bare `error!`. The app's configured error handler decides what happens. A component hook reaches the helper via `DeferredWorld::commands()`.
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
- Free items: a top-level `pub fn`, `pub const` or `pub static` is permitted only in a `*_ext` utility module, in a sanctioned namespace module (a coherent API called module-qualified, ie `js_runtime::cwd()`, allowlisted in the audit skill), as a `#[template]` constructor, or in generated/ABI-mandated code, ie proc-macro entry points. Everything else is an associated fn/const on the type it mainly relates to, or is not pub (`pub(crate)` and private free items are fine, the rule governs public API shape). Bevy systems and observers keep the free fn shape (the bevy idiom) but are private by default, registered by their plugin. Visibility is private until needed, for types as well as functions. Audit recipe: `.agents/skills/code-quality/audit-free-fns`.
- git: Whether on a branch, worktree or detacthed head, do not create branches or make commits unless explicitly told to. By default just keep things as unstaged changes.
- when the world has to do something like a one-off traversal, just use with_state, ie world.with_state::<(Resource<Foo>,Query<&Children..>)>(||{resource.bar});.
- never pass through bundles unnessecarily: fn default_router(bundle: impl Bundle)->impl Bundle ((bundle,Router)). it is pointless and obscures the function signature
- `.agents`: directory for files authored by users and agents, for agents
	- `.agents/plans`
	- `.agents/reports`
	- `.agents/skills`
	- `.agents/tmp`: scratchpads, output logs and dumps, wip scripts, etc
- Unless explcitly told to, never create extension methods on World, EntityRef, Commands or any of their async/mut counterparts. 
- Web APIs: reach for the ergonomic rust wrapper in `beet_core::web_utils`, don't hand-roll raw `wasm-bindgen`. A raw `Closure` handed to a browser API has to be kept alive for exactly as long as the API holds it and torn down after, which is where leaks and use-after-free come from; the wrappers own that lifetime in `Drop` and hand you a rust shape instead, ie `AnimationFrame` and `IntervalStream` are `Stream`s you `.next().await`, `HtmlEventListener` is a `Stream` of events, `ResizeListener` disconnects its observer on drop. Missing a wrapper is a reason to add one to `web_utils`, not to inline a `Closure::wrap` at the call site.
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

## Permissions

- 

## Testing


- We use the custom `beet_core::testing` test runner and matchers in all crates.
- All tests must use the beet core test attribute ie `#[beet_core::test]`. Inside `beet_core` itself write `#[crate::test]`: a crate that names itself turns its self dev-dependency into a real reference, linking a second copy of the crate into the lib test binary (see `crates/beet_core/Cargo.toml`).
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


## Beet CLI Cheatsheet

- when editing rust and using the beet cli run `cargo run -p beet-cli --features=feat1,feat2 -- arg1 arg2`
- when editing bsx files use the installed `beet` cli, ie `beet arg1 arg2`.
- when writing documentation, a runnable example command is plain `beet --main=..`: each entry declares its features with `<CrateCheck>`, so a copy-paste command needs no `beet --features=..`. Mention `--features` only to explain the verification mechanism, never in the runnable command itself.
- in the case of `beet --features`, this will check that beet has these features enabled and error if it doesnt.
- Install the beet cli via `cargo install --path crates/beet-cli --all-features`.
- `--main` accepts an entry file (`--main=examples/hello/main.bsx`) or a directory probed for `main.bsx` (`--main=examples/hello`); with no `--main` discovery walks the cwd and its ancestors.
- An entry that mounts paths outside its own directory declares `<StoreRoot src="../.."/>` (there is no `--root` flag): `src` names a position relative to the entry's location *in its store*, not a filesystem directory, so an fs store re-roots at the resolved ancestor while a self-rooted store (a bucket, browser storage) takes a key-prefix view and fails loudly when the root escapes the store, catching a store published from the entry's directory instead of its declared universe. Live reload watches the store's local root when it has one (the rebased root; a self-rooted store watches nothing), and command outputs (`dist/`, `site.pdf`) land beside the entry deliberately. An entry declares its required features with `<CrateCheck features={["thread", "sockets"]}/>`.
- Three browser binaries span the range: `just build-wasm-min` (`assets/wasm/beet-min.wasm`, the smallest binary that is still a beet runtime in the browser), `just build-wasm-render` (`assets/wasm/beet-render.wasm`, the render middle: the windowed wgpu stack in a tab, GPU via WebGPU when the browser grants an adapter, GPU-less otherwise) and `just build-wasm-full` (`assets/wasm/beet-full.wasm`, the kitchen sink: every feature a browser binary can boot with). Their feature sets are `web_min`/`web_render`/`web_full` in `crates/beet-cli/Cargo.toml`, whose comments carry the remaining exclusions and why; narrower sets in between get their own artifact rather than widening `min`. `build-wasm` itself is target-agnostic, so package/features/out are always explicit, never defaulted to a beet binary.

## Bevy Cheatsheet

- Observers can accept closures that accept their enviromnent, but systems cannot. Instead use input parameters: `fn my_system(foo: In<Foo>,...){}`;
- when spawning entities prefer to use world.spawn((ParentComponent,children![(ChildComponent,..)])) instead of calling spawn again for the child with ChildOf(), unless the child entity needs to be tracked for the test.
- Traversal. traversing entity hierarchies can quickly become a mess. for anything remotely complex just formalize it with a SystemParam, see `card_query.rs` for a good example of this. Avoid traversing using world directly, instead run a system, ie `world.run_system_once(|ancestors:Query<&ChildOf>| ... let root = ancestors.root(entity))`. also we have many existing traversal helpers ie AncestorQuery,
- often a world.with_state::<MyQuery>(|my_query|{}) is more ergonomic than world.run_system_once(|my_query:MyQuery|{..});
- Prefer Populated over Query which will skip system running if that query is empty, if its an 'any of these queries' pattern, use my_system.run_if(|a,b|!a.is_empty() || !b.is_empty()..)
- A `#[template]` is a constructor returning `impl Bundle`, not a UI/content-only thing. `#[template(system)]` takes `SystemParam`s (`Commands`, queries, resources) and can do arbitrary ECS work at build time, eg spawn child entities or inject routes. Prefer a `<MyThing/>` template over a bespoke reflect-marker + `On<Insert>` observer for markup-spawnable setup: it expands away at build, leaving no component to re-fire on scene reload.
- Templates may also return `()` for effects, or Result<impl Bundle> if fallible
- Component hooks: `#[component(on_add = ...)]` accepts a call yielding a closure, so use the constructors instead of a bespoke `fn on_add(world: DeferredWorld, cx: HookContext)`: `observe(my_observer)` / `observe((obs_a, obs_b))` registers observers watching the entity, `entity_hook(|entity| { ... })` runs any `EntityCommands` work. Both live in `beet_core::bevy_utils::hook_ext`.
- A command aimed at an entity whose lifetime **another task owns** (a server's connections, anything an async accept loop despawns) must tolerate its absence: `try_insert`, `try_remove`, `try_trigger_target`. An entity world scope does not flush, and `EntityWorldMut::despawn` flushes the queue itself *after* removing the entity, so an observer's deferred command routinely applies to a target that has since gone; under the default error handler that raise is a panic on the world thread.

## Action Cheatsheet

- An entity holds **at most one** action, and `ActionMeta` describes it (immutable, so every change is an insert `Insert<ActionMeta>` consumers can observe). `Action` is the only producer of `ActionMeta` (inserting it inserts the meta, removing it removes the meta); a second action with a different handler raises a clobber error rather than silently taking the slot.
- Extra signatures for the same behaviour go on an `ActionOverload<In, Out>`, which holds a full `Action<In, Out>` adapting the canonical action, registers its pair in `ActionMeta.overloads`, and delegates to the canonical action directly (never back through resolution). `ActionMeta::matches::<In, Out>()` is the single matching predicate.
- Resolution is **self-only**: `entity.call::<In, Out>(input)` takes the entity's canonical `Action<In, Out>`, else its `ActionOverload<In, Out>`. `ActionOf` / `Actions` mean agent targeting and nothing else.
- A provider (`ContinueRun`, `RunningSet`, `Router`, every `#[action]` component) guards its `#[require]`d action with `#[component(on_add = Action::<In, Out>::assert_provider::<Self>)]`, since `#[require]` silently yields to a colocated explicit component. Middleware (a `Next` in its input) claims no slot at all: it pushes onto the host's `MiddlewareList`.
- A long-running **facet** is one closure plus its selection, contributed to its entity's `RunningSet` from the facet component's `on_add` via `RunningSet::add(entity, label, select, func)`. `func` IS the facet: it starts the work, holds it open across a shutdown receiver, then tears it down. There is no stop action anywhere, stopping is signalling. The set owns the entity's single parked action: calling it parks a `Running<Out>`, fires `StartRunning<In>` for observers, then drives every facet `select` accepted concurrently under one local task. A facet that errors signals the survivors, awaits their teardown and fails the parked call with the collapsed errors; a start no facet selected fails it loudly naming every declared label. Both failures are opt-out through `ExcludeRunningErrors`: `NONE_STARTED` parks an empty start silently, `FACET_FAILED` logs the error, drops that facet and keeps the survivors serving, still failing the call once nothing is left alive. Neither is authored on the site, where a dead facet must restart the unit. Removing the `Running` (interrupt, reload, despawn) signals every live facet. Servers (`HttpServer`, `CliServer`, `TuiServer`, `ReplServer`, ...) are just facets, with no exceptions, and the dispatch host is their **child**, so one server reads as `<HttpServer><Router>..</Router></HttpServer>` and several as `<CallOnReady {(A, B)}><Router>..</Router></CallOnReady>`. `RunningSetFilter` owns the `--server` grammar every server's `select` closure reads.
- `exchange()` calls *this* entity's `Request -> Response` action; `exchange_child()` is the downward hop a server uses to reach the first child serving that pair.
- `CallOnReady` is the one load verb: on the entity's `Ready` it calls the entity's action with the process request and streams/exits. It tries `Request -> Response`, then `() -> Outcome` (`Pass` exits zero, `Fail` nonzero), then `() -> ()`, so a behavior scene is just `<Sequence {CallOnReady}>`. It is the tag when a root declares several servers (`<CallOnReady {(CliServer, HttpServer)}>`), a spread when one server is the tag (`<CliServer always=true {CallOnReady}>`). It fires on every load, so a file says exactly what happens when it is loaded, wherever it is loaded; a loader building a document to render or inspect rather than run (`check`, `export-static`, the Worker) disarms the subtree by inserting `DisableCallOnReady` on the root it builds, resolved by ancestry so later loads under a disarmed root (a rendered route page) inherit it. An explicit boot (`CallOnReady::call(entity, request)`) ignores the disarm: an explicit call is deliberate. There is no wrapper command that loads one entry into another's process and synthesizes a request for it: an entry that is its own CLI is launched directly and named its verb on argv (`beet --main=site serve --server=http`, `beet --main=site deploy`), which is the identical path the deployed unit's `ExecStart` takes.
- `CallOnStart` is the start verb, the other lifecycle edge, and the same shape as the load verb: `Ready` sweeps a loaded subtree, and a declared run start sweeps the started one. `StartRunning` rides a `ScopedTrigger`, which reads the event-typed `SweepDescendants<E>` marker off its target: a root declaring it sweeps the event over its subtree (deepest first, root last) and `CallOnStart` observes its own entity, calling its action with the start request, detached, trying `Request -> Outcome` then `() -> Outcome`. An undeclared start fires on its entity alone and the verb stays silent, deliberately: actions don't magically run. The sweep never leaves its root's subtree, so co-resident entries never start each other's work. `ServerPlugin` registers the `SweepDescendants<StartRunning<Request>>` instantiation, so an entry root authors `{SweepDescendants}` bare. `RunThread` is the reference user: it requires a `CallOnStart` and contributes an `ActionOverload<Request, Outcome>` adapting the behavior, so the thread's store adoption and scene reduce ride the entity's own action rather than a helper every caller must remember.

## BSX Cheatsheet

- **Every entity is authored under the tag of the type it most *is*.** `<div>`/`<span>` are UI elements and mean "this paints as a box of text"; they are never a generic carrier to hang the real type off a spread. This is not a UI-only rule, it applies to every position: a behavior loop is `<Repeat>`, a thread is `<Thread>`, a view is `<ThreadView>`, a route is `<Route>`. Whatever is left over rides a `{spread}` on that same entity.
	- bad: `<div {(Route{path:"deploy"}, ExchangeSequence)}>` / `<div {Repeat} {RunThread}>` / `<div {(ThreadView, OfThread($thread))}/>`
	- good: `<Route path="deploy" {ExchangeSequence}>` / `<Repeat {RunThread}>` / `<ThreadView {OfThread($thread)}/>`
	- picking between co-located types: the entity's **action** wins (one action per entity, see the Action Cheatsheet), so `<Repeat {RunThread}>`, not `<RunThread {Repeat}>`. Absent an action, the noun the entity names wins: `<Thread {(Sequence, FsStore{path:".."})}>`.
- A generic type resolves by base name to its sole registered instantiation, as a tag exactly as in a spread, so `<Repeat>`, `<Sequence>` and `<RepeatTimes total_times=2>` all author directly.
- When no component/resource/template fits a position (a plain grouping), use `<Fragment>`, not `<div>`: it carries spreads, directives and children but emits no element, the markup twin of returning `impl Bundle`. (`<Template>` is the *include* front-end, `<Template src="..">`; with no `src` it is a directives-only no-op that builds no children.)
- `<Tag/>` resolves a component/template by short type path and spawns its own entity; `{Spread}` / `{(A, B)}` adds components to the *current* entity. String attributes coerce to the field type (`SmolStr`, `SmolPath`, `Duration` from `"30s"`, an `Option<T>` wrapping the value, an enum unit variant by name), so a reflect component is usually authorable directly without a template.
- A `<Tag>`'s children land as its direct children (slots are transparent), so a child-reading handler like `{ExchangeSequence}` (a sequenced route) reads them: `<Route path="deploy" {ExchangeSequence}><MyBlock/><MyAction/></Route>`.
- **Features remove components, never entities.** A document loads whole in every binary; there are no feature-gating mechanisms. An uppercase tag nothing is registered under warns, marks its entity `UnregisteredTag` and still builds its directives, spreads and children, so a lean binary's tree has the same shape a full one's does and only behavior is missing. Loudness comes at dispatch, where it can tell a lean binary from a typo: a feature-dependent subtree declares `{RequireFeatures(["infra","extra"])}` and dispatching into it without those features fails naming them (the declaration also quiets the per-tag warn to `debug!`); a sequence route that skipped every child fails, with the reporting edge naming any unregistered tags among them; and `beet check` (which registers everything) elevates every `UnregisteredTag` to an error. `<CrateCheck features={[".."]}/>` is the load-time inverse, an entry *demanding* features and failing without them; `allow_unregistered` is the narrow opt-out for a tag whose whole content is the missing behavior (`<LiveReloadScript/>`), which resolves to nothing at all, children included.
- A `Router` is a **url space**: its subtree's routes are rooted at it, no ancestor `<Route>` segment prepends, and it owns its own `RouteTree`. So a whole site mounts under a command route (`<Route path="serve" {(HttpServer, TuiServer)}><Router>..`) with its urls still rooted at `/`, and a dispatching surface can never reach routes outside the namespace it serves. Resolve a tree from a held root with `RouteTree::of` (a server's tree lives on its `Router` child), not `entity.get::<RouteTree>()`.

## Cloud Resource Cheatsheet

- A cloud resource is declared ONCE, in markup, as its provider block (`<DynamoTableBlock bx:ref="analytics" label="analytics"/>`, `<S3BucketBlock label="app"/>`). Both meanings hang off that one entity: the deploy meaning (an `ErasedBlock`, always compiled) and the runtime meaning (a live store, attached by an `InfraPlugin` observer). Never derive a resource name a second time.
- **A resource belongs to the stack it is authored under, and to no other.** `StackQuery::declared` is the stack root's inclusive descendants, full stop: there is no adoption, no sweep that hands an unscoped block to whichever stack matches the process stage. A declaration outside every `<Stack>` still resolves the process default for its RUNTIME meaning, but no deploy provisions it. Load-bearing identity therefore rides `<Stack>`, a component registered in every native build, never a template prop: a prop is absent exactly in the binary that did not link the template, which is a lean boot silently resolving the wrong stage. A verb bundle carrying no identity is fine (`<DeployRoutes/>` = `Stack::verbs()`, the tofu lifecycle routes resolving their stack by ancestry).
- A declaration carries only its **label**, and identity is TWO types. `Stack` is the DECLARATION: a markup-authorable component whose `app_name`/`stage`/`region` are each optional, so `<Stack/>` bare works anywhere and `<Stack stage="shared"/>` overrides one field. `Stack::resolve(&PackageConfig)` yields `ResolvedStack`, which has no optional fields and is the ONLY type that composes `<app>--<stage>--<label>` (`resource_ident`, `resource_name`, `build_config`), so a name can never compose from a half-answered scope. Unset fields fall back to the nearest ancestor `<Stack>` else the process (`PackageConfig::app_name` + `BootstrapConfig::stage` + `AWS_REGION`); that resolve is the only in-world read of `AWS_REGION`, and every store is handed a resolved region rather than reaching for one. Reach both through `StackQuery` (`resolve`, `stack`, `root`, `deployment`). One-launch mechanics (the state backend, the work dir, the deploy id every artifact is keyed by) live beside it on the `Deployment` resource, which `InfraPlugin` inits so one launch publishes under one id.
- App identity lives in exactly one place, `PackageConfig::app_name`, which is always set (`DEFAULT_APP_NAME`, `"beet-app"`) and inserted by `BootstrapPlugin` at build time. A generic fallback is safe only because there is ONE composition, so both meanings of a declaration fall back identically; two independent derivations is the bug it guards. `Stack` overrides it only for a multi-app entry.
- A store (`S3Store`, `DynamoStore`) sits BELOW `Stack` in the crate graph, so it cannot resolve an ancestor stack and a region-less one in a tree would ignore it. Stack-relative resolution belongs to the DECLARATION (`<S3BucketBlock/>`, `<DynamoTableBlock/>`, `<DirSync/>`), whose attach observer hands the resolved region in; the only public store constructor makes a caller name a region.
- A RUNTIME consumer names the declaration through a relation, never a name: `<Router {(AnalyticsConfig, TableStoreRef($analytics))}>`. A consumer with nothing to point at is a loud spawn-time error, not a silent local fallback. A DEPLOY-time reference from one block to another is the exception and is a typed label handle (`VpcRef`, `DatabaseRef`, `SecurityGroupRef`, `LambdaRef`), because `Block::apply_to_config` sees its own entity and no world: the handle owns the ident composition both sides go through, so the resource name is still derived exactly once and a dangling reference fails the tofu plan naming the address.
- A recurring job is a declaration like any other: `<ScheduledJobBlock label="rollup-daily" target="rollup" schedule="cron(0 3 * * ? *)" path="analytics/rollup"/>` renders an EventBridge schedule plus an invoke role scoped to the one function its `LambdaRef` names, and declares no grants of its own (what the job may touch is the target lambda's own declared grants, lowered as usual). The expression is validated at render, so a typo fails the deploy rather than the job. What an invoke delivers is a `ScheduledInvoke` (`beet_net`), the ONE payload type the block serializes and the lambda adapter deserializes, tagged and versioned because a schedule keeps invoking with the payload the last deploy rendered: `lambda_http` hands the non-http event over as a pass-through, the adapter dispatches it through `exchange_child` exactly like an http request, and its status resolves the INVOCATION rather than a response body, since a job whose dispatch failed must fail the invoke or it reports green in every metric the schedule publishes.
- A block is a DECLARATION, not a sequence step, so nothing dispatches it during a deploy. Anything a block must *do* therefore belongs to whichever step consumes its output, never to the block being run: an artifact is built by the apply that uploads it (`TofuApplyAction` -> `BuildArtifact::build`), because a build wired as its own step is a build some other entry forgets, and an artifact uploaded but never built is a stale binary shipping under a green deploy.
- Permissions are declared by the resource (`Block::runtime_access` -> `AccessGrant`) and **lowered** by the compute block, which for the AWS computes is the shared `IamPolicy`: it seeds the statements the compute needs on its own account (an artifacts bucket, a log group), lowers the stack's grants into read/write bucket and per-table statements, and renders them in call order. A compute whose lowering yields nothing emits no policy resource at all, never a managed `FullAccess` one. (`StalwartBlock` is the one compute still rendering its own: it lowers the ssm parameter kind the shared core does not, and its blob store needs `s3:AbortMultipartUpload`.) A resource block never writes an ARN; a compute block never names a sibling resource. An `AccessGrant` is `{kind, name, permissions}`: `kind` is a plain string constant the declaring block owns (`S3BucketBlock::ACCESS_KIND`), so a new provider mints `"r2_bucket"` without touching shared code, and the ARN region comes from the compute's own resolved stack rather than the grant. Lowering is **loud on unknown**: a kind the compute cannot lower fails the deploy naming both the kind and the compute, and there are no `_ =>` catch-alls in grant handling (a silently dropped grant is a box that serves until the first request touching that resource).

## Styling Cheatsheet

- Colocate a widget's classes with the widget, not in a central rules file. A widget owns its styling.
- If a widget has only one class, use `inline_class!` rather than registering a named rule. A plain declaration is a `(prop, value)` pair; to point a prop at a design token use `Declaration::token(prop, value)`, ie `Declaration::token(BackgroundColor, colors::InverseSurface)`.
- Put the `inline_class!` in a helper function (eg `fn toast_style() -> impl Bundle`) when it is more than two tokens long; keep it inline at the call site otherwise.
