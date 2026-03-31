use beet_core::prelude::*;
use beet_tool::prelude::*;
use bevy::reflect::Typed;


pub fn function_tool<T, M>(
	path: &str,
	description: &str,
	tool: T,
) -> (ToolDefinition, Tool<T::In, T::Out>)
where
	T: IntoReflectTool<M>,
	T::In: Typed,
	T::Out: Typed,
{
	let meta = T::reflect_meta().input_json_schema();
	// println!("Registering tool {path} with meta {meta}");
	(
		FunctionToolDefinition::new(path, description, meta).into(),
		tool.into_tool(),
	)
}



#[derive(Debug, Clone, Component)]
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
/// output is returned as regular context
#[derive(Debug, Clone, Deref)]
pub struct ProviderToolDefinition {
	name: String,
}

impl ProviderToolDefinition {
	pub fn name(&self) -> &str { &self.name }
}

/// Tool defined in this application,
/// output is the result of a function call.
#[derive(Debug, Clone)]
pub struct FunctionToolDefinition {
	/// The name of the tool. This must be unique per set of tools.
	name: String,
	/// A description of the function. Used by the model to decide when to call it.
	description: String,
	/// A json schema for the parameters
	params_schema: serde_json::Value,
}
impl FunctionToolDefinition {
	pub fn new(
		name: impl Into<String>,
		description: impl Into<String>,
		params_schema: serde_json::Value,
	) -> Self {
		Self {
			name: name.into(),
			description: description.into(),
			params_schema,
		}
	}
	pub fn name(&self) -> &str { &self.name }
	pub fn description(&self) -> &str { &self.description }
	pub fn params_schema(&self) -> &serde_json::Value { &self.params_schema }
}




#[derive(Debug, Default, Clone, Component)]
pub enum ToolChoice {
	/// The agent may or may not select one of the available tools
	#[default]
	Auto,
	/// The agent may or may not select one of the listed tools
	AutoList(Vec<String>),
	/// The agent must select one of the available tools
	RequiredAny,
	/// The agent must select one of the listed tools
	RequiredList(Vec<String>),
	/// The agent must not select any tool
	None,
}
