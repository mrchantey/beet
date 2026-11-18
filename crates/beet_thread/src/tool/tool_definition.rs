// only the two `ActionMeta` readers below are action-aware; the definitions
// themselves are plain wire types.
#[cfg(feature = "action")]
use crate::prelude::HostRoute;
#[cfg(feature = "action")]
use beet_action::prelude::*;
use beet_core::prelude::*;
// `PathPattern`, read by the two action-gated readers below.
#[cfg(feature = "action")]
use beet_net::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// Rebuilds a route's [`ToolDefinition`] when its path lands, inserted with
/// its [`ActionMeta`].
#[cfg(feature = "action")]
pub(crate) fn insert_tool_definition(
	ev: On<Insert<PathPattern>>,
	commands: Commands,
	query: Query<
		(&ActionMeta, &PathPattern, Option<&RouteParams>),
		Without<HostRoute>,
	>,
) -> Result {
	refresh_tool_definition(ev.entity, commands, query)
}

/// Rebuilds a route's [`ToolDefinition`] when its declared params land, which
/// a request-reading route's schema is built from: each extractor's hook
/// queues its declaration, so it lands after the path.
#[cfg(feature = "action")]
pub(crate) fn refresh_tool_params(
	ev: On<Insert<RouteParams>>,
	commands: Commands,
	query: Query<
		(&ActionMeta, &PathPattern, Option<&RouteParams>),
		Without<HostRoute>,
	>,
) -> Result {
	refresh_tool_definition(ev.entity, commands, query)
}

#[cfg(feature = "action")]
fn refresh_tool_definition(
	entity: Entity,
	mut commands: Commands,
	query: Query<
		(&ActionMeta, &PathPattern, Option<&RouteParams>),
		Without<HostRoute>,
	>,
) -> Result {
	let Ok((meta, pattern, params)) = query.get(entity) else {
		return Ok(());
	};
	let params = params
		.into_iter()
		.flat_map(|params| params.iter())
		.flat_map(|declaration| declaration.metas())
		.collect::<Vec<_>>();
	// only routes shaped as a callable tool become a `FunctionToolDefinition`,
	// see `from_meta`: the observer fires on every route, but most routes are
	// not tools.
	if let Some(def) = FunctionToolDefinition::from_meta(meta, pattern, params)?
	{
		commands.entity(entity).insert(ToolDefinition::from(def));
	}
	Ok(())
}

#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize, Component)]
pub enum ToolDefinition {
	/// A tool defined in this application.
	Function(FunctionToolDefinition),
	/// A tool defined by the model provider.
	Provider(ProviderToolDefinition),
}

impl ToolDefinition {
	pub fn provider(name: impl Into<String>) -> Self {
		Self::Provider(ProviderToolDefinition { name: name.into() })
	}
	/// A function tool answered by the route at `path`, see
	/// [`FunctionToolDefinition::new`].
	pub fn function(
		path: impl Into<String>,
		description: impl Into<String>,
		params_schema: impl Into<JsonSchema>,
	) -> Result<Self> {
		FunctionToolDefinition::new(path, description, params_schema)
			.map(Self::Function)
	}
}

impl From<FunctionToolDefinition> for ToolDefinition {
	fn from(def: FunctionToolDefinition) -> Self { Self::Function(def) }
}

impl From<ProviderToolDefinition> for ToolDefinition {
	fn from(def: ProviderToolDefinition) -> Self { Self::Provider(def) }
}

/// Tool defined by the model provider,
/// output is returned as regular context.
#[derive(Debug, Clone, Deref, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct ProviderToolDefinition {
	name: String,
}

impl ProviderToolDefinition {
	pub fn name(&self) -> &str { &self.name }
}

/// Tool defined in this application,
/// output is the result of a function call.
///
/// A tool has one identity, the path of its route in the agent's url space,
/// the [`Router`](beet_router::prelude::Router) beneath the agent, ie
/// `read-blob`. A provider refuses a `/` in a function name, so the name it
/// sees is that path encoded ([`Self::encode_name`]) and a call decodes back
/// to it ([`Self::decode_name`]).
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct FunctionToolDefinition {
	/// The path of the route answering a call, in the agent's url space.
	path: String,
	/// A description of the function. Used by the model to decide when to call it.
	description: String,
	/// A json schema for the parameters.
	params_schema: JsonSchema,
}
impl FunctionToolDefinition {
	/// What a `/` in a route path becomes in a provider-facing name. Route
	/// segments are kebab case, so it never occurs in one.
	const SEPARATOR: &str = "__";
	/// The longest function name a provider accepts.
	const MAX_NAME_LEN: usize = 64;

	/// A tool answered by the route at `path`.
	///
	/// ## Errors
	///
	/// Errors when `path` cannot be named to a provider, which accepts
	/// `^[a-zA-Z0-9_-]{1,64}$`: a segment holding `__` or a character outside
	/// that set, or a path too long once encoded.
	pub fn new(
		path: impl Into<String>,
		description: impl Into<String>,
		params_schema: impl Into<JsonSchema>,
	) -> Result<Self> {
		let path = path.into();
		let name = Self::encode_name(&path);
		if path
			.split('/')
			.any(|segment| segment.contains(Self::SEPARATOR))
		{
			bevybail!(
				"the tool route `{path}` holds `{}`, which encodes a `/` in its provider name: rename the route",
				Self::SEPARATOR
			);
		}
		if name.is_empty()
			|| name.len() > Self::MAX_NAME_LEN
			|| !name
				.chars()
				.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
		{
			bevybail!(
				"the tool route `{path}` is named `{name}` to a provider, which accepts only 1 to {} of `a-z`, `A-Z`, `0-9`, `_` and `-`: shorten or rename the route",
				Self::MAX_NAME_LEN
			);
		}
		Self {
			path,
			description: description.into(),
			params_schema: params_schema.into(),
		}
		.xok()
	}
	/// The path of the route answering a call, in the agent's url space.
	pub fn path(&self) -> &str { &self.path }
	/// The function name a provider sees for the tool route at `path`, each
	/// `/` as `__`.
	pub fn encode_name(path: &str) -> String {
		path.replace('/', Self::SEPARATOR)
	}
	/// The tool route path a provider's function `name` encodes, inverting
	/// [`Self::encode_name`].
	pub fn decode_name(name: &str) -> String {
		name.replace(Self::SEPARATOR, "/")
	}
	pub fn description(&self) -> &str { &self.description }
	pub fn params_schema(&self) -> &JsonSchema { &self.params_schema }
	/// Mutable access to the parameter schema, eg to inject a runtime `enum`
	/// constraint on a field (see [`StringEnumOptions`](crate::prelude::StringEnumOptions)).
	pub fn params_schema_mut(&mut self) -> &mut JsonSchema {
		&mut self.params_schema
	}

	/// Build a tool definition from a route's metadata, or `None` if the route
	/// is not a callable tool: a wildcard/parameterized path (eg
	/// `run-wasm/*args`) cannot be a tool, and a route with no description is
	/// a plain route rather than an agent tool. The observer fires on every
	/// route, so this skips the non-tools instead of erroring.
	///
	/// A route reading its params off the request, its input a [`Request`]
	/// or [`RequestParts`], takes its tool arguments as a JSON body of those
	/// params, so its declared `params` are its input schema
	/// ([`ParamMeta::object_schema`]); any other route's input type is. A
	/// request-reading route whose path a provider cannot name is no tool,
	/// since most such routes are pages and files (`sitemap.xml`), never
	/// meant for an agent.
	///
	/// ## Errors
	///
	/// Errors when a typed-input tool route cannot be named to a provider
	/// ([`Self::new`]).
	#[cfg(feature = "action")]
	pub fn from_meta<'a>(
		meta: &ActionMeta,
		pattern: &PathPattern,
		params: impl IntoIterator<Item = &'a ParamMeta>,
	) -> Result<Option<Self>> {
		let (true, Some(description)) =
			(pattern.is_static(), meta.description())
		else {
			return Ok(None);
		};
		let path = pattern.annotated_path().to_string();
		if meta.input_is::<Request>() || meta.input_is::<RequestParts>() {
			return Self::new(
				path,
				description,
				ParamMeta::object_schema(params),
			)
			.ok()
			.xok();
		}
		match meta.input_json_schema()? {
			Some(schema) => Self::new(path, description, schema).map(Some),
			None => Ok(None),
		}
	}
}

#[derive(Debug, Default, Clone, Component, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize, Component, Default)]
pub enum ToolChoice {
	/// The agent may or may not select one of the available tools.
	#[default]
	Auto,
	/// The agent may or may not select one of the listed tools, each named by
	/// its route path.
	AutoList(Vec<String>),
	/// The agent must select one of the available tools.
	RequiredAny,
	/// The agent must select one of the listed tools, each named by its route
	/// path.
	RequiredList(Vec<String>),
	/// The agent must not select any tool.
	None,
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_router::prelude::*;

	fn tool(path: &str) -> Result<FunctionToolDefinition> {
		FunctionToolDefinition::new(path, "a tool", serde_json::json!({}))
	}

	/// Stands in for a verb whose flags are its tool arguments.
	#[action(route = "eval/grade")]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Grade(
		/// The eval to grade.
		#[param]
		eval: String,
		/// The level reached.
		#[param]
		level: Option<u8>,
		/// Grade again though graded.
		#[param]
		force: bool,
	) -> Result<String> {
		let _ = (level, force);
		Ok(eval)
	}

	/// A route reading its params off the request is a tool whose schema is
	/// those params, each with its type, doc line and whether it is required.
	#[beet_core::test]
	fn route_params_are_the_tool_schema() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<ThreadPlugin>();
		let router = app
			.world_mut()
			.spawn((Router, children![Grade::default()]))
			.flush();
		let route = app.world().entity(router).get::<Children>().unwrap()[0];
		app.update();
		let Some(ToolDefinition::Function(tool)) =
			app.world().entity(route).get::<ToolDefinition>()
		else {
			panic!("expected a function tool");
		};
		tool.path().xpect_eq("eval/grade");
		tool.description().xpect_contains("Stands in for a verb");
		let schema = Value::clone(tool.params_schema());
		schema.to_json_string().xpect_snapshot();
	}

	/// A provider sees the route path with each `/` as `__`, and a call's name
	/// decodes back to the route.
	#[beet_core::test]
	fn names_round_trip() {
		let name = FunctionToolDefinition::encode_name("store/read-blob");
		name.as_str().xpect_eq("store__read-blob");
		FunctionToolDefinition::decode_name(&name).xpect_eq("store/read-blob");
		tool("store/read-blob").unwrap();
	}

	/// A route a provider could not name refuses to become a tool, naming the
	/// route.
	#[beet_core::test]
	fn refuses_an_unnameable_route() {
		tool("read__blob")
			.unwrap_err()
			.to_string()
			.xpect_contains("`read__blob` holds `__`");
		tool(&"a".repeat(65))
			.unwrap_err()
			.to_string()
			.xpect_contains("shorten or rename the route");
		tool("read.blob")
			.unwrap_err()
			.to_string()
			.xpect_contains("`read.blob`");
	}
}
