//! The [`Script`] route surfaces: the typed [`ExchangeScript`] marker (a route
//! served from a sibling `Script`'s typed output) and the [`ScriptEntry`] entry
//! action (a `<script>` body run for its console output).
//!
//! Both are thin Request/Response wrappers; the eval machinery (typed runs and
//! console capture, for every backend) lives upstream on [`Script`] in
//! `beet_action`. This module only bridges a [`Request`] into a script `input` and
//! wraps the result in a [`Response`].

use crate::prelude::*;
use beet_action::prelude::Script;
use beet_action::prelude::ScriptAction;
use beet_action::prelude::ScriptLanguage;
use beet_core::prelude::*;
use beet_net::prelude::ExchangeAction;
use beet_net::prelude::FromRequest;
use beet_net::prelude::PathPartial;
use beet_net::prelude::RequestParts;
use beet_net::prelude::SerdeFromRequestMarker;
// the entry action and its request/body shaping are json-gated.
#[cfg(feature = "json")]
use beet_net::prelude::Request;
#[cfg(feature = "json")]
use beet_net::prelude::Response;
#[cfg(feature = "json")]
use beet_net::prelude::header;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

/// Runs the marked element's script body as a regular `Request -> Response`
/// action, capturing `console.log` into the response body (`console.error` to
/// stderr).
///
/// The "`node main.js`" entry: occupy an entry's action slot with it and the boot
/// ([`BootOnLoad`](beet_net::prelude::BootOnLoad)) calls it like any exchangeable
/// action, streaming the captured output. The script source is the marked
/// element's raw-text body, with the [`Request`] shaped into its `input`:
///
/// ```bsx
/// <script {(ScriptEntry, BootOnLoad)}>console.log("hello world")</script>
/// ```
///
/// The backend is the element's `language` attribute ([`ScriptLanguage::from_str`]),
/// falling back to the build default ([`ScriptLanguage::default`]) when absent:
///
/// ```bsx
/// <script language="rhai" {(ScriptEntry, BootOnLoad)}>print("hello world")</script>
/// ```
///
/// Being async, it awaits the full request body and includes it in the `input` (so
/// a `POST` body reaches the script at `input.body`). The console-capture machinery
/// is [`Script::run_captured`]; this action only reads the element text/attributes
/// and shapes the request into the script `input`. The sibling of the typed
/// [`ExchangeScript`] route (which serves a `Script`'s typed output instead of its
/// console).
///
/// Gated on `json`: the request `input` is marshalled through beet's [`Value`]. The
/// underlying [`Script::run_captured`] is backend-agnostic (rhai or quickjs).
#[cfg(feature = "json")]
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn ScriptEntry(cx: ActionContext<Request>) -> Result<Response> {
	let entity = cx.id();
	// the element's raw-text body and its `language`, read together from the world.
	let (script, language) = cx
		.world()
		.with_state::<(ElementTextQuery, AttributeQuery), _>(
			move |(elements, attributes)| {
				let script = elements.text_content(entity);
				// fall back to the build default when the attribute is absent or
				// names an unavailable language.
				let language = attributes
					.find(entity, "language")
					.and_then(|(_, value)| value.as_str().ok())
					.and_then(|name| name.parse::<ScriptLanguage>().ok())
					.unwrap_or_default();
				(script, language)
			},
		)
		.await;
	if script.trim().is_empty() {
		return Response::ok().xok();
	}
	let input = request_input(cx.take()).await?;
	let body = Script::<Value, ()>::new(language, script).run_captured(input)?;
	Response::ok().with_body(body).xok()
}

/// The request as the script's `input`: a `{ path, params, body }` [`Value`] map.
///
/// The request body is awaited and bound at `input.body`, as a string or bytes:
/// the `content-type` decides first (a text media type is a string, otherwise
/// bytes); with no `content-type` the bytes are a string if they are valid UTF-8,
/// else bytes.
#[cfg(feature = "json")]
async fn request_input(request: Request) -> Result<Value> {
	let path = request
		.path_string()
		.split('/')
		.filter(|segment| !segment.is_empty())
		.map(Value::str)
		.xmap(Value::new_list);
	let params = request
		.params()
		.iter_all()
		.map(|(key, values)| {
			let values =
				Value::new_list(values.iter().map(|value| value.as_str()));
			(key.clone(), values)
		})
		.collect::<Map>();
	let body = request_body(request).await?;
	let mut input = Map::default();
	input.insert("path", path);
	input.insert("params", Value::Map(params));
	input.insert("body", body);
	Value::Map(input).xok()
}

/// The request body as a [`Value`], a string or bytes per the `content-type` (or
/// UTF-8 validity when none is present). See [`request_input`].
#[cfg(feature = "json")]
async fn request_body(request: Request) -> Result<Value> {
	let content_type = request
		.headers
		.get::<header::ContentType>()
		.and_then(|res| res.ok());
	let bytes = request.body.into_bytes().await?;
	match content_type {
		// a declared text type is decoded as UTF-8 (lossily, never failing).
		Some(media_type) if media_type.is_text() => {
			Value::str(String::from_utf8_lossy(&bytes).into_owned())
		}
		// a declared non-text type stays bytes.
		Some(_) => Value::Bytes(bytes.to_vec()),
		// no type: a string if valid UTF-8, else bytes.
		None => match String::from_utf8(bytes.to_vec()) {
			Ok(text) => Value::str(text),
			Err(err) => Value::Bytes(err.into_bytes()),
		},
	}
	.xok()
}

/// Reflect-able marker that installs the typed [`ScriptAction`] and the
/// type-erased [`ExchangeAction`] for a [`Script<Input, Output>`] route.
///
/// Serves the script's typed [`Output`](Script) (eg a `String` the script
/// returns), not its console output (that is [`ScriptEntry`]). `M1`/`M2` are
/// [`FromRequest`]/[`ExchangeRouteOut`] markers. The defaults handle the serde
/// blanket case; for custom extractors (eg [`QueryParams`], [`RequestParts`])
/// instantiate as `ExchangeScript::<Input, Output, _, _>` and let inference pick
/// them.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[reflect(where)]
#[require(
	ScriptAction<Input, Output>,
	ExchangeAction = TransformExchange::new::<Input, Output, M1, M2>(),
)]
pub struct ExchangeScript<
	Input = (),
	Output = (),
	M1 = SerdeFromRequestMarker,
	M2 = SerdeIntoResponseMarker,
> where
	Input: 'static + Send + Sync + Serialize + FromRequest<M1>,
	Output: 'static + Send + Sync + DeserializeOwned + ExchangeRouteOut<M2>,
	M1: 'static + Send + Sync,
	M2: 'static + Send + Sync,
{
	#[reflect(ignore)]
	_marker: PhantomData<fn() -> (Input, Output, M1, M2)>,
}

impl<Input, Output, M1, M2> Default for ExchangeScript<Input, Output, M1, M2>
where
	Input: 'static + Send + Sync + Serialize + FromRequest<M1>,
	Output: 'static + Send + Sync + DeserializeOwned + ExchangeRouteOut<M2>,
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
	Output: 'static + Send + Sync + DeserializeOwned + ExchangeRouteOut<M2>,
	M1: 'static + Send + Sync,
	M2: 'static + Send + Sync,
{
	fn clone(&self) -> Self { Self::default() }
}

impl<Input, Output, M1, M2> std::fmt::Debug
	for ExchangeScript<Input, Output, M1, M2>
where
	Input: 'static + Send + Sync + Serialize + FromRequest<M1>,
	Output: 'static + Send + Sync + DeserializeOwned + ExchangeRouteOut<M2>,
	M1: 'static + Send + Sync,
	M2: 'static + Send + Sync,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ExchangeScript").finish()
	}
}

/// A markup-friendly scripted route: a `path` plus a `script` over the request
/// parts, serving the script's string output as the response.
///
/// The non-generic front-end for a `(PathPartial, Script, ExchangeScript)` route,
/// so a no-code entry declares one without spelling the generic types:
///
/// ```bsx
/// <ScriptRoute path="add"
///   script='"result: " + (Number(input.url.params.a[0]) + Number(input.url.params.b[0]))'/>
/// ```
///
/// The script runs in the build's default backend ([`ScriptLanguage::default`]),
/// so a quickjs binary runs JavaScript with the request as its `input` (a quickjs
/// `RequestParts` exposes the query params at `input.url.params`).
#[template]
pub fn ScriptRoute(
	#[prop(into)] path: String,
	#[prop(into)] script: String,
) -> impl Bundle {
	(
		PathPartial::new(path),
		Script::<RequestParts, String>::new(ScriptLanguage::default(), script),
		ExchangeScript::<RequestParts, String, _, _>::default(),
	)
}

/// An `ExchangeScript` route installs the typed `ScriptAction` (hence an
/// `ActionMeta`) and the `ExchangeAction`, so the script's output is served as the
/// route response. Regression: requiring only `Script` left the route without an
/// `ActionMeta`, so it never joined the `RouteTree`.
#[cfg(test)]
#[cfg(feature = "rhai")]
mod route_test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn exchange_script_route_dispatches() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((default_router(), children![(
				Script::<(), String>::rhai(r#""hello world""#),
				ExchangeScript::<(), String>::default(),
				PathPartial::new("greet"),
			)]))
			.exchange(Request::get("greet"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("hello world");
	}
}

/// `ScriptEntry` is a regular exchangeable action: routed with a request, it runs
/// the element's script body and returns its console output as the body. Tested
/// through the quickjs backend (the json-bearing backend in the test matrix),
/// whose `console.log` is the stdout channel.
#[cfg(test)]
#[cfg(all(feature = "quickjs", not(target_arch = "wasm32")))]
mod entry_test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn script_entry_captures_console() {
		AsyncPlugin::world()
			.spawn((ExchangeAction(ScriptEntry.into_action()), children![
				Value::Str(r#"console.log("hi")"#.into())
			]))
			.exchange_str(Request::get("/"))
			.await
			.xpect_eq("hi\n".to_string());
	}

	/// The awaited request body is bound at `input.body`: a `POST` with a plain text
	/// body (no `content-type`) decodes as a UTF-8 string the script can echo.
	#[beet_core::test]
	async fn script_entry_reads_body() {
		AsyncPlugin::world()
			.spawn((ExchangeAction(ScriptEntry.into_action()), children![
				Value::Str(r#"console.log(input.body)"#.into())
			]))
			.exchange_str(Request::post("/").with_body("hello body"))
			.await
			.xpect_eq("hello body\n".to_string());
	}

	/// The `language` attribute selects the backend: a `language="rhai"` element runs
	/// its body through rhai (a `print`, which the default quickjs backend would
	/// reject) rather than the build default. Spawns the attribute as a related
	/// [`AttributeOf`] entity, mirroring parsed markup.
	#[cfg(feature = "rhai")]
	#[beet_core::test]
	async fn script_entry_reads_language_attribute() {
		let mut world = AsyncPlugin::world();
		let element = world
			.spawn((ExchangeAction(ScriptEntry.into_action()), children![
				Value::Str(r#"print("from rhai")"#.into())
			]))
			.id();
		world.spawn((
			AttributeOf::new(element),
			Attribute::new("language"),
			Value::Str("rhai".into()),
		));
		world
			.entity_mut(element)
			.exchange_str(Request::get("/"))
			.await
			.xpect_eq("from rhai\n".to_string());
	}
}
