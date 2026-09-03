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

A [`Script`] is a JavaScript program carried as data, transforming `Input` into `Output`. It holds a program, not an engine: the backend is chosen at compile time from the target and the `quickjs` feature, and a build with no usable backend errors when run rather than silently degrading.

Every script is the body of an async function, so `await` is legal anywhere and a script answers with `return`. What it returns is deserialized into `Output`, and a script that returns nothing is a `null`: `()` and `Value` accept that, a typed output rejects it naming the one way to answer.

`Script` is the program and its sibling `ScriptConfig` is the envelope: `world` and `console` (whether each global is installed at all), `read`/`write` (what the world bridge will address on the script's behalf) and `limits` (time, memory and interpreter stack, each host-realm backend documenting whichever it cannot enforce). The config is non-generic, so a policy can be stamped or audited without knowing the script's types, and an absent sibling is the default: everything on, open filters. A withheld global is simply not installed, so touching it throws an ordinary catchable `ReferenceError` — "pure" is not a mode, it is `world: false`.

Every `world` method is *served live*: the script sends one call and awaits a promise, the host performs the operation against the world and answers, and that reply settles the promise. So a read returns current state, a write lands immediately and in order, `world.spawn` resolves to a real entity id the next line can use, and a refused call rejects where it was made, catchable in the script.

Every backend serves through one `WorldBridge`, and every operation is async: it takes exclusive world access for as long as it needs and gives it back, so a step that is legitimately asynchronous (a schema asking something beyond the world before it will accept a value) runs with nothing held. `WorldRead` and `WorldWrite` are the synchronous `&mut World` halves those sections call. The embedded engine runs its whole evaluation in a local task, because its runtime is `!Send` and an `#[action]` future is not; the result comes back over a oneshot. A script making a dozen calls still settles inside one sync point, so `<Repeat>` over a `<RunScript>` is a tick, not a call per frame.

The wire is beet's `Value`, not JSON, in both directions and at the reflect boundary: `ValueSerializer`/`ValueDeserializer` are real serde data formats, so a registered component crosses without a JSON hop and a runtime one carries what it holds. The transport encoding is unchanged, since every backend is a JavaScript host.

`Script` installs no action of its own, so a domain action can carry one beside its own without fighting over the entity's `ActionMeta`. The markers are what make it callable: `OutcomeScript` (and its `<RunScript script=".."/>` template) is the behaviour-tree leaf, so `<Repeat>` over it is a ticking dynamic system; `ExchangeScript` (and its `<ScriptRoute path=".." script=".."/>` template, in beet_router) is the same script serving a route, handed the request as `input` and answering with what it returned — a string as plain text, anything else as JSON. Both address components by type path (short or full) and both take a sibling `ScriptConfig` spread.

The `read`/`write` halves of a config are `GlobFilter`s, open by default. A component passes a filter if any of its names matches an include and none matches an exclude, so `"Name"` reaches the canonical `bevy_ecs::name::Name` while `exclude:["secrets.*"]` cannot be sidestepped by spelling it the other way. Independent of any filter, the bridge refuses writes to `ScriptConfig` itself and to the script carriers, so a script can never widen its own grant — toggles and limits included. Enforcement is host-side at the bridge, per call; only the two toggles and the limits ever cross the wire to a backend.

`<DynamicComponent name="guestbook.Visits" schema="u64"/>` mints a component type with no rust definition behind it, holding a `Value` — whatever a document field can hold — so a scene adds vocabulary the engine never shipped and a script reads and writes it exactly as it does a registered one. The `schema` is a `ValueSchema` and defaults to `Any`, which accepts anything; declare one and every write is validated before it reaches the component's storage, so a rejection reaches the script as the same catchable error a refusal is. The markup form is the kind (`"u64"`, `"bytes"`, ..) or a JSON Schema object; anything richer is a rust expression. The name is the identity, so a second declaration carrying a *different* schema is an error rather than a last writer.

Because a schema can coerce, a script can hold bytes: JSON has no byte type, so they cross as a list of numbers and a `bytes` schema restores the type at the destination, the way every other coercion in beet works. `world.schema` therefore answers a runtime component with its declaration, which is a contract, where a registered component still gets a shallow `TypeInfo` sketch, which is a courtesy.

The `world` JavaScript is one shared source every backend splices, naming one host hook per direction which the embedded engine and each host realm install their own way, so the transports cannot drift.
