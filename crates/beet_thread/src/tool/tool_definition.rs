use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_tool::prelude::*;
use serde::Deserialize;
use serde::Serialize;

pub(crate) fn insert_tool_definition(
	// path pattern is inserted by ToolMeta
	ev: On<Insert, PathPattern>,
	mut commands: Commands,
	query: Query<(&ToolMeta, &PathPattern)>,
) -> Result {
	let tool = query.get(ev.entity)?;
	let def: ToolDefinition = FunctionToolDefinition::from_meta(tool)?.into();
	commands.entity(ev.entity).insert(def);

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
		params_schema: serde_json::Value,
	) -> Self {
		Self::Function(FunctionToolDefinition::new(
			name,
			description,
			params_schema,
		))
	}
}

impl Into<ToolDefinition> for FunctionToolDefinition {
	fn into(self) -> ToolDefinition { ToolDefinition::Function(self) }
}

impl Into<ToolDefinition> for ProviderToolDefinition {
	fn into(self) -> ToolDefinition { ToolDefinition::Provider(self) }
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
	params_schema: JsonValue,
}
impl FunctionToolDefinition {
	pub fn new(
		path: impl Into<String>,
		description: impl Into<String>,
		params_schema: serde_json::Value,
	) -> Self {
		Self {
			path: path.into(),
			description: description.into(),
			params_schema: JsonValue(params_schema),
		}
	}
	pub fn path(&self) -> &str { &self.path }
	pub fn description(&self) -> &str { &self.description }
	pub fn params_schema(&self) -> &serde_json::Value { &self.params_schema }

	pub fn from_meta((meta, path): (&ToolMeta, &PathPattern)) -> Result<Self> {
		if !path.is_static() {
			bevybail!(
				"Tool path must be static (no parameters or wildcards) to create a FunctionToolDefinition.\nPath provided: {path}"
			);
		}
		let path = path.annotated_rel_path().to_string();
		let description = meta
			.description()
			.ok_or_else(||{
				bevyhow!("ToolMeta lacks description, which is required to create a FunctionToolDefinition.\n{meta:?}")
			})?;
		let params_schema = meta.input_json_schema().ok_or_else(||{
			bevyhow!("ToolMeta lacks input json schema, which is required to create a FunctionToolDefinition.\n{meta:?}")
		})?;
		Ok(Self::new(path, description, params_schema))
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
