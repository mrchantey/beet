use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde::Deserialize;
use serde::Serialize;

pub(crate) fn insert_tool_definition(
	// path pattern is inserted by ActionMeta
	ev: On<Insert, PathPattern>,
	mut commands: Commands,
	query: Query<(&ActionMeta, &PathPattern)>,
) -> Result {
	let tool = query.get(ev.entity)?;
	// only routes shaped as a callable tool — a static path with a description and
	// an input schema — become a `FunctionToolDefinition`. Other routes (wildcard or
	// parameterized paths like `run-wasm/*args`, and system routes with no tool
	// metadata) are not agent-callable tools, so they are skipped rather than erroring:
	// the observer fires on every route, but most routes are not tools.
	if let Some(def) = FunctionToolDefinition::from_meta(tool)? {
		commands.entity(ev.entity).insert(ToolDefinition::from(def));
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
	pub fn function(
		name: impl Into<String>,
		description: impl Into<String>,
		params_schema: impl Into<Schema>,
	) -> Self {
		Self::Function(FunctionToolDefinition::new(
			name,
			description,
			params_schema,
		))
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
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct FunctionToolDefinition {
	/// The path to the tool. This must be unique per set of tools.
	path: String,
	/// A description of the function. Used by the model to decide when to call it.
	description: String,
	/// A json schema for the parameters.
	params_schema: Schema,
}
impl FunctionToolDefinition {
	pub fn new(
		path: impl Into<String>,
		description: impl Into<String>,
		params_schema: impl Into<Schema>,
	) -> Self {
		Self {
			path: path.into(),
			description: description.into(),
			params_schema: params_schema.into(),
		}
	}
	pub fn path(&self) -> &str { &self.path }
	pub fn description(&self) -> &str { &self.description }
	pub fn params_schema(&self) -> &Schema { &self.params_schema }

	/// Build a tool definition from a route's metadata, or `None` if the route is
	/// not a callable tool: a wildcard/parameterized path (eg `run-wasm/*args`)
	/// cannot be a tool, and a route with no description or input schema is a plain
	/// route rather than an agent tool. The observer fires on every route, so this
	/// skips the non-tools instead of erroring.
	pub fn from_meta(
		(meta, path): (&ActionMeta, &PathPattern),
	) -> Result<Option<Self>> {
		// a tool needs a concrete callable path with a description and input schema.
		let (true, Some(description), Some(params_schema)) = (
			path.is_static(),
			meta.description(),
			meta.input_json_schema(),
		) else {
			return Ok(None);
		};
		Ok(Some(Self::new(
			path.annotated_path().to_string(),
			description,
			params_schema,
		)))
	}
}

#[derive(Debug, Default, Clone, Component, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize, Component, Default)]
pub enum ToolChoice {
	/// The agent may or may not select one of the available tools.
	#[default]
	Auto,
	/// The agent may or may not select one of the listed tools.
	AutoList(Vec<String>),
	/// The agent must select one of the available tools.
	RequiredAny,
	/// The agent must select one of the listed tools.
	RequiredList(Vec<String>),
	/// The agent must not select any tool.
	None,
}
