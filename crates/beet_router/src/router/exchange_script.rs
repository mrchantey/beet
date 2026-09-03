//! The [`Script`] route surfaces: the [`ExchangeScript`] marker (a route served
//! from a sibling `Script`'s output) and the [`ExchangeScriptElement`] entry
//! action (a `<script>` body run for its console output).
//!
//! Both are thin Request/Response wrappers; the eval machinery (the world
//! bridge, the console capture, and the compile-time backend selection behind
//! them) lives upstream on [`Script`] in `beet_action`. This module only bridges
//! a [`Request`] into a script `input` and wraps the result in a [`Response`],
//! so it registers and compiles whether or not a backend is present.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

/// Runs the marked element's script body as a regular `Request -> Response`
/// action, capturing `console.log` into the response body (`console.error` to
/// stderr).
///
/// The "`node main.js`" entry: occupy an entry's `Action<Request, Response>` slot
/// with it and [`CallOnReady`](beet_net::prelude::CallOnReady) calls that
/// slot once the entry loads, streaming the captured output. (A script element
/// installs a plain `Action<Request, Response>`, which the load verb calls
/// directly.) The script source is the marked element's
/// raw-text body, with the [`Request`] shaped into its `input`:
///
/// ```bsx
/// <script {(ExchangeScriptElement, CallOnReady)}>console.log("hello world")</script>
/// ```
///
/// Being async, it awaits the full request body and includes it in the `input` (so
/// a `POST` body reaches the script at `input.body`). An element script is
/// world-capable like any other, bounded by an optional sibling
/// [`ScriptConfig`]; what it *returns* is discarded, since the console is the
/// body here. The sibling of the typed [`ExchangeScript`] route (which serves a
/// `Script`'s output instead of its console).
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn ExchangeScriptElement(
	cx: ActionContext<Request>,
) -> Result<Response> {
	let entity = cx.id();
	let world = cx.world().clone();
	// the element's raw-text body and the grant it runs under, read together.
	let (script, config) = world
		.with_state::<(ElementTextQuery, Query<&ScriptConfig>), _>(
			move |(elements, configs)| {
				(
					elements.text_content(entity),
					configs.get(entity).ok().cloned(),
				)
			},
		)
		.await;
	if script.trim().is_empty() {
		return Response::ok().xok();
	}
	let input = match <Value as FromRequest<ScriptInputMarker>>::from_request(
		cx.take(),
	)
	.await
	{
		Ok(input) => input,
		Err(response) => return Ok(response),
	};
	let body = Script::<Value, Value>::new(script)
		.run_captured(input, world, &config.unwrap_or_default())
		.await?;
	Response::ok().with_body(body).xok()
}

/// Reflect-able marker that installs the typed [`ScriptAction`], its
/// [`ScriptConfig`], and the [`ExchangeOverload`] adapting them to
/// request/response dispatch.
///
/// Serves what the script returned (eg a `String`), not its console output (that
/// is [`ExchangeScriptElement`]). `M1`/`M2` are
/// [`FromRequest`]/[`IntoResponseWithRequestParts`] markers. The defaults handle
/// the serde blanket case; for custom extractors (eg [`QueryParams`],
/// [`RequestParts`], or the `Value` pair [`ScriptRoute`] uses) instantiate as
/// `ExchangeScript::<Input, Output, _, _>` and let inference pick them.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[reflect(where)]
#[require(
	ScriptAction<Input, Output>,
	ScriptConfig,
	ExchangeOverload = route::exchange_overload::<Input, Output, M1, M2>(),
)]
pub struct ExchangeScript<
	Input = (),
	Output = (),
	M1 = SerdeFromRequestMarker,
	M2 = SerdeIntoResponseMarker,
> where
	Input: 'static + Send + Sync + Serialize + FromRequest<M1>,
	Output: 'static
		+ Send
		+ Sync
		+ DeserializeOwned
		+ IntoResponseWithRequestParts<M2>,
	M1: 'static + Send + Sync,
	M2: 'static + Send + Sync,
{
	#[reflect(ignore)]
	_marker: PhantomData<fn() -> (Input, Output, M1, M2)>,
}

impl<Input, Output, M1, M2> Default for ExchangeScript<Input, Output, M1, M2>
where
	Input: 'static + Send + Sync + Serialize + FromRequest<M1>,
	Output: 'static
		+ Send
		+ Sync
		+ DeserializeOwned
		+ IntoResponseWithRequestParts<M2>,
	M1: 'static + Send + Sync,
	M2: 'static + Send + Sync,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<Input, Output, M1, M2> Clone for ExchangeScript<Input, Output, M1, M2>
where
	Input: 'static + Send + Sync + Serialize + FromRequest<M1>,
	Output: 'static
		+ Send
		+ Sync
		+ DeserializeOwned
		+ IntoResponseWithRequestParts<M2>,
	M1: 'static + Send + Sync,
	M2: 'static + Send + Sync,
{
	fn clone(&self) -> Self { Self::default() }
}

impl<Input, Output, M1, M2> std::fmt::Debug
	for ExchangeScript<Input, Output, M1, M2>
where
	Input: 'static + Send + Sync + Serialize + FromRequest<M1>,
	Output: 'static
		+ Send
		+ Sync
		+ DeserializeOwned
		+ IntoResponseWithRequestParts<M2>,
	M1: 'static + Send + Sync,
	M2: 'static + Send + Sync,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ExchangeScript").finish()
	}
}

/// A markup-friendly scripted route: a `path` plus a `script` served the whole
/// request and answering with what it returns.
///
/// The non-generic front-end for a `(PathPartial, Script, ExchangeScript)`
/// route, so a no-code entry declares one without spelling the generic types:
///
/// ```bsx
/// <ScriptRoute path="sign"
///   {ScriptConfig{read:["Name"]}}
///   script="
///     const name = (input.params.name || [''])[0];
///     if (!name) return 'a name is required';
///     await world.spawn({ Name: name });
///     return 'signed: ' + name;
///   "/>
/// ```
///
/// The request is the script's `input`, a `{ path, params, body }` map, and the
/// world is reached through the `world` global, so there is no reply channel
/// beyond `return`. A string answers as plain text, anything else as JSON, and a
/// script that returned nothing answers JSON `null`. What it may reach rides
/// alongside as a [`ScriptConfig`] spread.
#[template]
pub fn ScriptRoute(
	/// The url path this route mounts at.
	#[prop(into)]
	path: String,
	/// The JavaScript source, an async function body evaluated with the request
	/// bound to `input` and the `world` API installed.
	#[prop(into)]
	script: String,
) -> impl Bundle {
	(
		PathPartial::new(path),
		Script::<Value, Value>::new(script),
		ExchangeScript::<Value, Value, ScriptInputMarker, ScriptAnswerMarker>::default(),
	)
}

/// A `ExchangeScript` route installs the typed `ScriptAction` (hence an
/// `ActionMeta`) and the `ExchangeOverload` adapter, so what the script returns is
/// served as the route response. Regression: requiring only `Script` left the route
/// without an `ActionMeta`, so it never joined the `RouteTree`.
#[cfg(test)]
#[cfg(feature = "quickjs")]
mod route_test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A router world whose registry knows [`Name`], the component these scripts
	/// read and write.
	fn world() -> World {
		let world = (AsyncPlugin, RouterPlugin).into_world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Name>();
		world
	}

	/// Dispatch a built route entity as the `Request -> Response` action it is,
	/// returning its body.
	async fn call(
		world: &mut World,
		route: Entity,
		request: Request,
	) -> Result<String> {
		world
			.entity_mut(route)
			.call::<Request, Response>(request)
			.await?
			.unwrap_str()
			.await
			.xok()
	}

	#[beet_core::test]
	async fn exchange_script_route_dispatches() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((Router::with_defaults(), children![(
				Script::<(), String>::new(r#"return "hello world""#),
				ExchangeScript::<(), String>::default(),
				PathPartial::new("greet"),
			)]))
			.exchange(Request::get("greet"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("hello world");
	}

	/// The markup front-end builds a working route: `<ScriptRoute>` expands to the
	/// `(PathPartial, Script, ExchangeScript)` triple, so the spawned entity serves
	/// what its script returns over the request it is dispatched with.
	#[beet_core::test]
	async fn script_route_serves_its_script() {
		let mut world = world();
		let route = world
			.spawn_template(rsx! {
				<ScriptRoute path="greet"
					script={r#"return "hello " + input.params.name[0]"#}/>
			})
			.unwrap()
			.id();
		call(&mut world, route, Request::get("greet?name=world"))
			.await
			.unwrap()
			.xpect_contains("hello world");
	}

	/// The route branches on the request: the rejecting branch answers with its
	/// message and spawns nothing, the accepting branch spawns and reads back
	/// what it just wrote.
	#[beet_core::test]
	async fn a_route_rejects_then_spawns() {
		let mut world = world();
		let route = world
			.spawn_template(rsx! {
				<ScriptRoute path="sign"
					{ScriptConfig::new(["Name"])}
					script={r#"
						const name = (input.params.name || [""])[0];
						if (!name) return "a name is required";
						const entry = await world.spawn({ Name: name });
						return "signed: " + (await world.get(entry, "Name"));
					"#}/>
			})
			.unwrap()
			.id();
		call(&mut world, route, Request::get("sign"))
			.await
			.unwrap()
			.xpect_eq("a name is required".to_string());
		world.query::<&Name>().iter(&world).count().xpect_eq(0);

		call(&mut world, route, Request::get("sign?name=ada"))
			.await
			.unwrap()
			.xpect_eq("signed: ada".to_string());
		world
			.query::<&Name>()
			.iter(&world)
			.next()
			.unwrap()
			.as_str()
			.xpect_eq("ada");
	}

	/// A non-string answer is JSON, so a script can hand back a structure.
	#[beet_core::test]
	async fn a_structured_answer_is_json() {
		let mut world = world();
		world.spawn(Name::new("ada"));
		let route = world
			.spawn_template(rsx! {
				<ScriptRoute path="book"
					{ScriptConfig::new(["Name"])}
					script={r#"
						const found = [];
						for (const id of await world.entities("Name")) {
							found.push(await world.get(id, "Name"));
						}
						return { names: found };
					"#}/>
			})
			.unwrap()
			.id();
		call(&mut world, route, Request::get("book"))
			.await
			.unwrap()
			.xpect_eq(r#"{"names":["ada"]}"#.to_string());
	}

	/// A script that returns nothing still answers, so a caller can tell
	/// "nothing to say" from "no answer".
	#[beet_core::test]
	async fn a_valueless_script_answers_null() {
		let mut world = world();
		let route = world
			.spawn_template(rsx! {
				<ScriptRoute path="sign"
					script={r#"await world.spawn({ Name: "ada" });"#}/>
			})
			.unwrap()
			.id();
		call(&mut world, route, Request::get("sign"))
			.await
			.unwrap()
			.xpect_eq("null".to_string());
	}

	/// The reach is enforced at the bridge, not in the script, so a route's
	/// refusal rejects where the script made the call and the script may answer
	/// with it.
	#[beet_core::test]
	async fn a_refused_write_is_catchable_in_the_script() {
		let mut world = world();
		world.spawn(Name::new("ada"));
		let route = world
			.spawn_template(rsx! {
				<ScriptRoute path="sign"
					{ScriptConfig::new(["Name"]).read_only()}
					script={r#"
						const [entry] = await world.entities("Name");
						try {
							await world.insert(entry, "Name", "bob");
							return "changed";
						} catch (err) { return err.message; }
					"#}/>
			})
			.unwrap()
			.id();
		call(&mut world, route, Request::get("sign"))
			.await
			.unwrap()
			.xpect_contains("may not write");
		world
			.query::<&Name>()
			.iter(&world)
			.next()
			.unwrap()
			.as_str()
			.xpect_eq("ada");
	}

	/// A scripted route is a route, not just an action: mounted under a `Router`
	/// it joins the `RouteTree` and answers a dispatched request at its path, one
	/// script writing the world and the other reading it back.
	#[beet_core::test]
	async fn the_routes_dispatch_from_a_router() {
		let mut world = world();
		let router = world
			.spawn((Router::with_defaults(), children![
				(
					route::new(
						"sign",
						Script::<Value, Value>::new(
							r#"
							await world.spawn({ Name: input.params.name[0] });
							return "signed";
							"#,
						),
					),
					ExchangeScript::<
						Value,
						Value,
						ScriptInputMarker,
						ScriptAnswerMarker,
					>::default(),
					ScriptConfig::new(["Name"]),
				),
				(
					route::new(
						"book",
						Script::<Value, Value>::new(
							r#"
							const found = [];
							for (const id of await world.entities("Name")) {
								found.push(await world.get(id, "Name"));
							}
							return found.join(",");
							"#,
						),
					),
					ExchangeScript::<
						Value,
						Value,
						ScriptInputMarker,
						ScriptAnswerMarker,
					>::default(),
					ScriptConfig::new(["Name"]),
				),
			]))
			.id();
		world
			.entity_mut(router)
			.exchange(Request::get("sign?name=ada"))
			.await
			.unwrap_str()
			.await
			.xpect_eq("signed".to_string());
		world
			.entity_mut(router)
			.exchange(Request::get("book"))
			.await
			.unwrap_str()
			.await
			.xpect_eq("ada".to_string());
	}
}

/// `ExchangeScriptElement` is a regular exchangeable action: routed with a request,
/// it runs the element's script body and returns its console output as the body.
/// Tested through the quickjs backend (the json-bearing backend in the test matrix),
/// whose `console.log` is the stdout channel.
#[cfg(test)]
#[cfg(feature = "quickjs")]
mod entry_test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn script_entry_captures_console() {
		AsyncPlugin::world()
			.spawn((ExchangeScriptElement, children![Value::Str(
				r#"console.log("hi")"#.into()
			)]))
			.call::<Request, Response>(Request::get("/"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_eq("hi\n".to_string());
	}

	/// The awaited request body is bound at `input.body`: a `POST` with a plain text
	/// body (no `content-type`) decodes as a UTF-8 string the script can echo.
	#[beet_core::test]
	async fn script_entry_reads_body() {
		AsyncPlugin::world()
			.spawn((ExchangeScriptElement, children![Value::Str(
				r#"console.log(input.body)"#.into()
			)]))
			.call::<Request, Response>(
				Request::post("/").with_body("hello body"),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_eq("hello body\n".to_string());
	}

	/// An element script reaches the world like any other, bounded by its
	/// sibling config: the console is the body, the world is the effect.
	#[beet_core::test]
	async fn script_entry_reaches_the_world() {
		let mut world = AsyncPlugin::world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Name>();
		world
			.spawn((
				ExchangeScriptElement,
				ScriptConfig::new(["Name"]),
				children![Value::Str(
					r#"
					const entry = await world.spawn({ Name: "ada" });
					console.log(await world.get(entry, "Name"));
					"#
					.into()
				)],
			))
			.call::<Request, Response>(Request::get("/"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_eq("ada\n".to_string());
	}
}
