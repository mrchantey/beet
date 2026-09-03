---
name: audit-free-fns
description: Audit top-level pub items (fns, consts, statics) against the beet free-item rule. Use when sweeping visibility (pub to pub(crate)/private) or relocating free items onto types, namespace modules and *_ext modules. Contains the detection recipe, the category walk, the namespace-module allowlist and the sweep discipline.
---

# Audit free items

The rule (AGENTS.md Conventions): a top-level `pub fn`, `pub const` or `pub static` is permitted only in a `*_ext` utility module, in a sanctioned namespace module (below), as a `#[template]` constructor, or in generated/ABI-mandated code. Everything else becomes an associated fn/const on the type it mainly relates to, or loses its `pub`. The rule governs public API shape: `pub(crate)` and private free items are tolerated. Visibility is private until needed, for types as well as functions.

Other item kinds need no rule: types, traits, type aliases and `macro_rules!` are inherently top-level, and their visibility is covered by private-until-needed.

## Detection

Zero indent means module-level: associated fns and consts sit indented inside `impl` blocks, so this grep finds the free items:

```sh
rg -n '^pub ((async |unsafe )?fn|const|static) ' crates --glob '*.rs' --glob '!*_ext.rs' --glob '!crates/beet_ui/src/style/material/classes/*'
```

- rg respects .gitignore, so generated code under `**/codegen` is excluded automatically.
- A crate with no `[lib]` target has no public API, so its items are out of the rule's scope (`beet-cli/src/main.rs` is the bin; `beet-cli/src/lib.rs` and everything it declares is not).
- Known gap: items inside inline `pub mod foo { }` blocks are indented, so the grep misses them. Rare under beet's file-per-module convention, but a clean grep is not proof of a clean crate.
- Templates are detected by the preceding line, not the name: rerun with `-B1` and check for `#[template`, which covers both `#[template]` and `#[template(system)]`. A PascalCase name is a corroborating signal only.

## Namespace modules

A module qualifies as a namespace when it is a coherent API over one domain and callers read best module-qualified, ie `js_runtime::cwd()`, the `std::fs`/`std::mem` idiom. This is distinct from a `_ext` grab-bag of utilities. The bar is high: most modules holding a stray free fn are not namespaces, they are missing an associated fn. Sanctioned:

- `beet_core/src/web_utils/js_runtime.rs` — `js_runtime::cwd()`
- `beet_core/src/terminal/escape.rs` — `escape::RESET`, `escape::cursor_goto(..)`: raw ANSI/VT100 sequences, already called module-qualified at every site
- `beet_thread/src/streaming/completions_mapper.rs` and `beet_thread/src/streaming/o11s_mapper.rs` — `completions_mapper::response_to_partial(..)`: one module per provider wire format, and the two deliberately share fn names, which only module-qualification disambiguates
- `beet_router/src/scene_routes/render_action.rs` — `render_action::pure_route(path, handler)`: the render-route constructors, one per handler kind, called module-qualified in- and cross-crate
- `beet_router/src/router/route.rs` — `route::new(path, bundle)`, `route::exchange(..)`: the Rust twin of the `<Route>` template. A route is a child of a router, not a router, so hanging these off `Router` reads as dissonant; the namespace is the fix
- `beet_net/src/server/stream_sniff.rs` — `stream_sniff::write_and_close(..)`: classify an accepted connection and answer it, already the call idiom at every site
- `beet_net/src/mdns/wire.rs` — `wire::build_ptr_query(..)`, `wire::MDNS_PORT`: the mDNS wire format (constants + codec), one layer of the protocol
- `beet_infra/src/terra/tofu.rs` — `tofu::apply(&dir)`: one fn per `tofu` subcommand, already the call idiom at every site
- `beet-cli/src/entry_build.rs` — `entry_build::resolve_main(..)`: the entry resolve/read/build pipeline, one coherent step set

To sanction a new one, add it here in the same changeset with the call-site idiom as justification.

## Categories

Walk each hit through these in order, first match wins:

1. **Test-support code** (`tests/` dirs, shared test utils, `beet_core/src/testing/`): skip, DRY test helpers outrank namespacing, and the custom test runner's entry points are ABI-shaped.
2. **`#[template]` constructor**: exempt entirely, the free fn is the template idiom.
3. **Action, middleware or handler**: exempt on the same footing. An action is `#[action(..)]` + `#[derive(Component)]` on a PascalCase fn (`SequenceAction`, `HighestScoreAction`); middleware is a plain fn taking a `Next<In, Out>` and used as a value (`trace_action.wrap(..)`); a handler is a plain fn over `ActionContext` passed as a value to `Action::new_*` (`default_renderer`, `post_streamer_action`). All are authoring vocabulary, and the fn shape is load-bearing.
4. **Proc-macro entry point** (`#[proc_macro]`, `#[proc_macro_derive]`, `#[proc_macro_attribute]`, ie `beet_core/macros`): exempt, the proc-macro ABI mandates a free fn.
5. **Bevy system, observer or plugin fn** (SystemParam or `On`/`Trigger` parameters, registered via `add_systems`/`add_observer`/`observe`; or `fn(&mut App)` passed to `add_plugins`): exempt from relocation, the fn shape is the bevy idiom. Visibility-only: private by default, its plugin registers it. Keep `pub` only when another crate registers or pipes it.
6. **Sanctioned namespace module** (allowlist above): exempt.
7. **`crates/beet_ui/src/style/material/classes/`**: untouched entirely, parked pending BSN (master plan item 15).
8. **Everything else**: relocate to an associated fn/const on the type it mainly relates to (a `DEFAULT_FOO` const almost always belongs on `Foo`); if genuinely typeless, into an existing or new `*_ext` module, or nominate a namespace module. Independently, drop visibility to `pub(crate)` or private unless consumed cross-crate, part of the authoring vocabulary (below), or obviously public API.

## Sweep discipline

- Visibility sweeps and relocation sweeps are separate changesets, never mixed: a visibility diff touches keywords only, a relocation diff moves and renames only, no signature or behavior changes in either.
- Work in per-crate chunks, compiler-verified after each chunk.
- "Not consumed cross-crate" is proven by lowering to `pub(crate)` and compiling, not by grep alone: preludes re-export widely, and the compiler errors on a `pub use` of a crate-private item.
- The compiler is not the only oracle. Beet is a framework with no external users yet, so "nothing in the workspace consumes this" does not imply private. Components, templates, actions and servers *intended* as user-authorable vocabulary (authorable in bsx markup, scenes or docs) stay pub without a cross-crate consumer. Intent is the test, not the kind: an internal bookkeeping component follows private-until-needed like anything else.
