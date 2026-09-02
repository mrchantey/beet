//! The world-bridged route surface: a script that serves a route *and* reaches
//! the [`World`].
//!
//! `beet_action`'s [`DynamicScript`] is a behaviour-tree leaf, `() -> Outcome`,
//! so a sequence route skips it: it serves nothing. This is the same script in
//! `Request -> Response` form. The request becomes the script's `input`, the
//! value it returns becomes the response body, and everything in between (the
//! `world` bridge, the exposure enforcement, the live executor) stays the
//! machinery `beet_action` already owns, called rather than restated.
//!
//! ```bsx
//! <DynamicScriptRoute path="sign" script=".." {ScriptExposure{read:["Name"]}}/>
//! ```

use super::exchange_script::request_input;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A markup-friendly world-bridged route: a `path` plus a `script` that acts on
/// the world through the `world` API.
///
/// The non-generic front-end for a `(PathPartial, DynamicScriptExchange)` route,
/// so a no-code entry mounts one without spelling the pieces:
///
/// ```bsx
/// <DynamicScriptRoute path="sign"
///   {ScriptExposure{read:["Name"]}}
///   script="
///     const name = (input.params.name || [''])[0];
///     if (!name) return 'a name is required';
///     await world.spawn({ Name: name });
///     return 'signed: ' + name;
///   "/>
/// ```
#[template]
pub fn DynamicScriptRoute(
	/// The url path this route mounts at.
	#[prop(into)]
	path: String,
	/// The JavaScript source, evaluated with the request bound to `input` and
	/// the `world` API installed.
	#[prop(into)]
	script: String,
) -> impl Bundle {
	(PathPartial::new(path), DynamicScriptExchange::new(script))
}

/// The request/response form of [`DynamicScript`]: a script that receives the
/// request and acts on the world through the `world` API.
///
/// The world is reached through the `world` global as usual, so `input` is just
/// the request, the same shape every scripted route in this crate shares:
///
/// ```json
/// { "path": ["sign"], "params": { "name": ["ada"] }, "body": "" }
/// ```
///
/// The route answers with what the script returned: a string is a plain-text
/// body (so `'signed: ada'` reads back verbatim), anything else is JSON, and a
/// script that returned nothing answers `null`. There is no reply channel,
/// because the script already has one, and a bridged script is an async
/// function body, so `return` is how it uses it.
///
/// The sibling [`ScriptExposure`] is the limit on what it may reach, exactly as
/// on the leaf action.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
#[require(DynamicScriptExchangeAction, ScriptExposure)]
pub struct DynamicScriptExchange {
	/// The JavaScript source, evaluated with the request bound to `input` and
	/// the `world` API installed.
	pub script: String,
	/// The resources the script may consume before it is cut off.
	pub limits: ScriptLimits,
}

impl DynamicScriptExchange {
	/// Create a [`DynamicScriptExchange`] from JavaScript source.
	pub fn new(script: impl Into<String>) -> Self {
		Self {
			script: script.into(),
			limits: ScriptLimits::default(),
		}
	}
}

/// Serves the caller's [`DynamicScriptExchange`]: evaluates the script with the
/// `world` bridge installed and answers with what it returned.
///
/// ## Errors
/// Errors if the caller has no [`DynamicScriptExchange`] or if the script fails
/// to evaluate. A refused call is not an error here: it rejects inside the
/// script, which may catch it and answer anyway.
#[action(handler_only)]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn DynamicScriptExchangeAction(
	cx: ActionContext<Request>,
) -> Result<Response> {
	let entity = cx.id();
	let world = cx.world();
	let (script, exposure) = world
		.with_state::<Query<(&DynamicScriptExchange, &ScriptExposure)>, _>(
			move |scripts| {
				scripts
					.get(entity)
					.map(|(script, exposure)| {
						(script.clone(), exposure.clone())
					})
					.ok()
			},
		)
		.await
		.ok_or_else(|| {
			bevyhow!(
				"DynamicScriptExchange caller {entity:?} has no DynamicScriptExchange"
			)
		})?;

	let input = request_input(cx.input).await?;
	Script::<Value, ()>::new(script.script)
		.with_limits(script.limits)
		.run_world(input, world.clone(), exposure)
		.await?
		.xmap(script_response)
}

/// What a script returned, as a response.
///
/// A string answers as plain text, so `'signed: ada'` reads back verbatim over
/// http and cli alike; anything else answers as JSON, and a script that returned
/// nothing answers JSON `null` rather than an empty body, so a caller can tell
/// "nothing to say" from "no answer".
///
/// # Errors
/// Propagates the JSON encoding failure of a non-string answer.
fn script_response(answer: Option<Value>) -> Result<Response> {
	match answer {
		Some(Value::Str(text)) => {
			Response::ok().with_media(MediaBytes::new_text(text.to_string()))
		}
		answer => Response::ok().with_media(MediaBytes::new_json(
			serde_json::to_string(&answer.unwrap_or(Value::Null))?,
		)),
	}
	.xok()
}

#[cfg(test)]
#[cfg(feature = "quickjs")]
mod test {
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

	/// The route branches on the request: the rejecting branch answers with its
	/// message and spawns nothing, the accepting branch spawns and reads back
	/// what it just wrote.
	#[beet_core::test]
	async fn a_route_rejects_then_spawns() {
		let mut world = world();
		let route = world
			.spawn_template(rsx! {
				<DynamicScriptRoute path="sign"
					{ScriptExposure::new(["Name"])}
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
				<DynamicScriptRoute path="book"
					{ScriptExposure::new(["Name"])}
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
				<DynamicScriptRoute path="sign"
					script={r#"await world.spawn({ Name: "ada" });"#}/>
			})
			.unwrap()
			.id();
		call(&mut world, route, Request::get("sign"))
			.await
			.unwrap()
			.xpect_eq("null".to_string());
	}

	/// Exposure is enforced at the bridge, not in the script, so a route's
	/// refusal rejects where the script made the call and the script may answer
	/// with it.
	#[beet_core::test]
	async fn a_refused_write_is_catchable_in_the_script() {
		let mut world = world();
		world.spawn(Name::new("ada"));
		let route = world
			.spawn_template(rsx! {
				<DynamicScriptRoute path="sign"
					{ScriptExposure::new(["Name"]).read_only()}
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

	/// The front-end is a route, not just an action: mounted under a `Router` it
	/// joins the `RouteTree` and answers a dispatched request at its path, one
	/// script writing the world and the other reading it back.
	#[beet_core::test]
	async fn the_routes_dispatch_from_a_router() {
		let mut world = world();
		let router = world
			.spawn((Router::with_defaults(), children![
				(
					DynamicScriptExchange::new(
						r#"
						await world.spawn({ Name: input.params.name[0] });
						return "signed";
					"#,
					),
					PathPartial::new("sign"),
					ScriptExposure::new(["Name"]),
				),
				(
					DynamicScriptExchange::new(
						r#"
						const found = [];
						for (const id of await world.entities("Name")) {
							found.push(await world.get(id, "Name"));
						}
						return found.join(",");
					"#,
					),
					PathPartial::new("book"),
					ScriptExposure::new(["Name"]),
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
