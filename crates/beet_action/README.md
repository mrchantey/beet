# beet_action

Functions as entities.

An [`Action`] component turns an entity into a function: `call` runs its handler with an input and awaits the output. Control-flow nodes like [`Sequence`] are just actions that call their children, so behavior trees, state machines and utility AI are all built from the same primitive.

An entity holds **at most one** action, and [`ActionMeta`] describes it: `Action` is the only producer of that descriptor (`ActionMeta` is immutable, so every change is an insert consumers can observe), and a second action with a different handler raises rather than silently taking the slot. Resolution is self-only, so `entity.call::<In, Out>(input)` never wanders into a relationship: it takes the entity's canonical `Action<In, Out>`, else an [`ActionOverload<In, Out>`] adapting that canonical action to another signature. `ActionMeta::matches::<In, Out>()` is the one matching predicate, used by call resolution, [`Sequence`] child validation, and the downward child selector. `ActionOf` / `Actions` mean agent targeting and nothing else.

A provider (a component that `#[require]`s an action, ie `ContinueRun`, [`RunningSet`], every `#[action]` component) guards its slot with `#[component(on_add = Action::<In, Out>::assert_provider::<Self>)]`, since `#[require]` silently yields to a colocated explicit component. Middleware (a `Next` in its input) claims no slot at all: it pushes onto the host's `MiddlewareList`.

```rust
# use beet_core::prelude::*;
# use beet_action::prelude::*;
# async fn run() -> Result {
// spawn actions as components, then call the entity
let outcome = AsyncPlugin::world()
	.spawn((Sequence::new(), children![
		Log::new("hello"),
		Log::new("world"),
	]))
	.call::<(), Outcome>(())
	.await?;
# Ok(()) }
```

The `#[action]` macro produces an action from a plain function in one of three flavors:

- `#[action(pure)]` runs synchronously with no world access. Fast.
- `#[action]` is a Bevy system: it takes its input as `In<T>` and may use any system param.
- `async fn` with `#[action]` gets async world access, so a handler can `await` other actions, IO or timers.

```rust,ignore
// pure: input in, output out
#[action(pure)]
#[derive(Component)]
fn Add(cx: ActionContext<(i32, i32)>) -> i32 { cx.0 + cx.1 }

// system: world access via system params
#[action]
fn CountNames(_cx: In<()>, names: Query<&Name>) -> usize { names.iter().count() }

// async: await other actions, IO or timers
#[action]
#[derive(Component)]
async fn Greet(cx: ActionContext<String>) -> String {
	format!("Hello, {}!", cx.value())
}
```

## Long-running work: facets

A long-running **facet** is one closure plus its selection, contributed to its entity's [`RunningSet`] from the facet component's `on_add` via `RunningSet::add(entity, label, select, func)`. `func` IS the facet: it starts the work, holds it open across a shutdown receiver, then tears it down. There is no stop action anywhere, stopping is signalling: removing the parked `Running` (interrupt, reload, despawn) signals every live facet.

The set owns the entity's single parked action: calling it parks a `Running<Out>`, fires `StartRunning<In>` for observers, then drives every facet the start's `select` accepted concurrently under one local task. A facet that errors signals the survivors, awaits their teardown and fails the parked call with the collapsed errors; a start no facet selected fails it loudly naming every declared label. Both failures are opt-out through [`BypassRunningErrors`], which logs and degrades instead, still failing the call once nothing is left alive.

Servers (`HttpServer`, `CliServer`, `TuiServer`, ...) are just facets, with no exceptions: see `beet_net` for the server topology and the lifecycle verbs (`CallOnReady`, `CallOnStart`) that call these actions.

## Scripts and the world bridge

A [`Script`] is a pure `Input -> Output` transformation whose body is JavaScript. It holds a program, not an engine: the backend is chosen at compile time from the target and the `quickjs` feature, and a build with no usable backend errors when run rather than silently degrading. `ScriptLimits` bound time, memory and interpreter stack, and a host-realm backend documents whichever of those its host cannot enforce.

World access is opt-in surface rather than ambient authority: `Script::run` installs no `world` global at all. `Script::run_world` does, and every method on it is *served live* — the script sends one call and awaits a promise, the host performs the operation against the world and answers, and that reply settles the promise. So a read returns current state, a write lands immediately and in order, `world.spawn` resolves to a real entity id the next line can use, and a refused call rejects where it was made, catchable in the script.

A bridged script is an async function body rather than an expression, which is what makes those `await`s legal, so it answers with `return`. `<DynamicScript>` is the behaviour-tree leaf that carries one, so `<Repeat>` over it is a ticking dynamic system; `<DynamicScriptRoute>` (in beet_router) is the same script serving a route, answering with what it returned. Both address components by type path (short or full) and both take a sibling `ScriptExposure`.

`ScriptExposure` is two `GlobFilter`s, `read` and `write`, open by default. A component passes a filter if any of its names matches an include and none matches an exclude, so `"Name"` reaches the canonical `bevy_ecs::name::Name` while `exclude:["secrets.*"]` cannot be sidestepped by spelling it the other way. Independent of any filter, the bridge refuses writes to the exposure itself and to the script carriers, so a script can never widen its own grant.

`<DynamicComponent name="guestbook.Visits"/>` mints a component type with no rust definition behind it, holding a `Value` — whatever a document field can hold — so a scene adds vocabulary the engine never shipped and a script reads and writes it exactly as it does a registered one.

The `world` JavaScript is one shared source every backend splices, naming one host hook per direction which the embedded engine and each host realm install their own way, so the transports cannot drift.
