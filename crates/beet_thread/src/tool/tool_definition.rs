use beet_core::prelude::*;



#[derive(Debug, Clone, Component)]
pub enum ToolDefinition {
	/// A tool defined in this application.
	Function(FunctionTool),
	/// A tool defined by the model provider.
	Provider(ProviderTool),
}

impl ToolDefinition {
	pub fn provider(name: impl Into<String>) -> Self {
		Self::Provider(ProviderTool { name: name.into() })
	}
	pub fn function(
		name: impl Into<String>,
		description: impl Into<String>,
		params_schema: serde_json::Value,
	) -> Self {
		Self::Function(FunctionTool::new(name, description, params_schema))
	}
}

impl Into<ToolDefinition> for FunctionTool {
	fn into(self) -> ToolDefinition { ToolDefinition::Function(self) }
}

impl Into<ToolDefinition> for ProviderTool {
	fn into(self) -> ToolDefinition { ToolDefinition::Provider(self) }
}

/// Tool defined by the model provider,
/// output is returned as regular context
#[derive(Debug, Clone, Deref)]
pub struct ProviderTool {
	name: String,
}

impl ProviderTool {
	pub fn name(&self) -> &str { &self.name }
}

/// Tool defined in this application,
/// output is the result of a function call.
#[derive(Debug, Clone)]
pub struct FunctionTool {
	/// The name of the tool. This must be unique per set of tools.
	name: String,
	/// A description of the function. Used by the model to decide when to call it.
	description: String,
	/// A json schema for the parameters
	params_schema: serde_json::Value,
}
impl FunctionTool {
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
