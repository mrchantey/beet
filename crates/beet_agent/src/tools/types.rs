//! Tool types for agent tool calling.
//!
//! This module provides the relationship types for connecting agents to their
//! available tools. Each agent can have a `Tools` relationship pointing to
//! tool entities that define available functions the model can call.
//!
//! # Architecture
//!
//! Tools work similarly to [`ThreadContext`](crate::prelude::ThreadContext):
//! - Each agent gets a `Tools` / `ToolOf` relationship
//! - Each `ToolOf` entity is a `router_exchange` with its own `EndpointTree`
//! - When exposing tools to the model API, we prefix route names with the entity ID
//! - When executing tools, we use `Entity::from_bits` to find the right tool set
//!
//! # Example
//!
//! ```ignore
//! world.spawn((
//!     Agent,
//!     related![Tools,
//!         tool_exchange((Sequence, children![
//!             EndpointBuilder::post()
//!                 .with_path("add")
//!                 .with_request_body(BodyMeta::json::<AddRequest>())
//!                 .with_response_body(BodyMeta::json::<AddResponse>())
//!                 .with_description("Add two numbers together")
//!                 .with_action(|Json(req): Json<AddRequest>| {
//!                     Json(AddResponse { result: req.a + req.b })
//!                 }),
//!         ]))
//!     ],
//!     related![Children,
//!         Action1,
//!         Action2,
//!     ]
//! ));
//! ```

use crate::prelude::*;
use beet_core::prelude::*;
use beet_router::prelude::*;

/// Points to the agent that owns this tool set.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Tools)]
pub struct ToolOf(pub Entity);

/// An ordered collection of tool sets this agent has access to.
/// Each tool set entity should have an `EndpointTree` component
/// describing the available tools.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = ToolOf, linked_spawn)]
pub struct Tools(Vec<Entity>);

/// Metadata for a tool definition, extracted from an `Endpoint`.
/// This is used to convert endpoints to the model API format.
#[derive(Debug, Clone)]
pub struct ToolMeta {
	/// The unique name for this tool (includes entity prefix for disambiguation)
	pub name: String,
	/// The original endpoint path (without entity prefix)
	pub path: String,
	/// Description of what the tool does
	pub description: Option<String>,
	/// The entity ID of the tool set this belongs to
	pub tool_set_entity: Entity,
	/// JSON schema for the request parameters
	pub parameters: Option<serde_json::Value>,
}

impl ToolMeta {
	/// Creates a tool name by combining the entity bits with the path.
	/// Format: `{entity_bits}_{path_with_underscores}`
	pub fn tool_name(entity: Entity, path: &str) -> String {
		let bits = entity.to_bits();
		let sanitized_path = path
			.trim_start_matches('/')
			.replace('/', "_")
			.replace(':', "")
			.replace('*', "");
		if sanitized_path.is_empty() {
			format!("tool_{bits}")
		} else {
			format!("tool_{bits}_{sanitized_path}")
		}
	}

	/// Parses a tool name back into entity and path components.
	/// Returns `None` if the name doesn't match the expected format.
	pub fn parse_tool_name(name: &str) -> Option<(Entity, String)> {
		let name = name.strip_prefix("tool_")?;
		let underscore_pos = name.find('_');

		let (bits_str, path) = match underscore_pos {
			Some(pos) => {
				let (bits, rest) = name.split_at(pos);
				(bits, rest.trim_start_matches('_').replace('_', "/"))
			}
			None => (name, String::new()),
		};

		let bits: u64 = bits_str.parse().ok()?;
		let entity = Entity::from_bits(bits);
		Some((entity, format!("/{path}")))
	}

	/// Converts this tool meta to a `FunctionToolParam` for the model API.
	pub fn to_function_tool_param(&self) -> openresponses::FunctionToolParam {
		let mut param = openresponses::FunctionToolParam::new(&self.name);

		if let Some(desc) = &self.description {
			param = param.with_description(desc);
		}

		if let Some(params) = &self.parameters {
			param = param.with_parameters(params.clone());
		}

		param
	}

	/// Creates a `ToolMeta` from an endpoint and its owning entity.
	pub fn from_endpoint(endpoint: &Endpoint, tool_set_entity: Entity) -> Self {
		let path = endpoint.path().annotated_route_path().to_string();
		let name = Self::tool_name(tool_set_entity, &path);

		// Convert request body schema to JSON Schema format
		let parameters = endpoint
			.request_body()
			.schema()
			.map(|schema| body_meta_to_json_schema(schema));

		Self {
			name,
			path,
			description: endpoint.description().map(|s| s.to_string()),
			tool_set_entity,
			parameters,
		}
	}
}

/// Converts a `TypeSchema` from `BodyMeta` to a JSON Schema object.
fn body_meta_to_json_schema(schema: &TypeSchema) -> serde_json::Value {
	let mut properties = serde_json::Map::new();
	let mut required = Vec::new();

	for field in schema.fields() {
		let field_schema = field_to_json_schema(field);
		properties.insert(field.name().to_string(), field_schema);

		if field.is_required() {
			required.push(serde_json::Value::String(field.name().to_string()));
		}
	}

	let mut obj = serde_json::Map::new();
	obj.insert("type".to_string(), serde_json::Value::String("object".to_string()));
	obj.insert("properties".to_string(), serde_json::Value::Object(properties));

	if !required.is_empty() {
		obj.insert("required".to_string(), serde_json::Value::Array(required));
	}

	serde_json::Value::Object(obj)
}

/// Converts a `FieldSchema` to a JSON Schema property.
fn field_to_json_schema(field: &FieldSchema) -> serde_json::Value {
	let type_path = field.type_path();

	// Map common Rust types to JSON Schema types
	let json_type = if type_path.contains("String") || type_path.contains("str") {
		"string"
	} else if type_path.contains("i8")
		|| type_path.contains("i16")
		|| type_path.contains("i32")
		|| type_path.contains("i64")
		|| type_path.contains("u8")
		|| type_path.contains("u16")
		|| type_path.contains("u32")
		|| type_path.contains("u64")
		|| type_path.contains("isize")
		|| type_path.contains("usize")
	{
		"integer"
	} else if type_path.contains("f32") || type_path.contains("f64") {
		"number"
	} else if type_path.contains("bool") {
		"boolean"
	} else if type_path.contains("Vec<") || type_path.contains("[") {
		"array"
	} else {
		"object"
	};

	serde_json::json!({ "type": json_type })
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn tool_name_generation() {
		let entity = Entity::from_bits(42);
		let name = ToolMeta::tool_name(entity, "/add");
		assert!(name.starts_with("tool_"));
		assert!(name.contains("add"));
	}

	#[test]
	fn tool_name_roundtrip() {
		let entity = Entity::from_bits(42);
		let original_path = "/users/create";
		let name = ToolMeta::tool_name(entity, original_path);

		let (parsed_entity, parsed_path) =
			ToolMeta::parse_tool_name(&name).unwrap();

		// Entity bits should match
		assert_eq!(entity.to_bits(), parsed_entity.to_bits());
		// Path should be reconstructed (may differ slightly due to sanitization)
		assert!(parsed_path.contains("users"));
		assert!(parsed_path.contains("create"));
	}

	#[test]
	fn tool_name_empty_path() {
		let entity = Entity::from_bits(123);
		let name = ToolMeta::tool_name(entity, "/");
		let (parsed_entity, _) = ToolMeta::parse_tool_name(&name).unwrap();
		assert_eq!(entity.to_bits(), parsed_entity.to_bits());
	}
}
