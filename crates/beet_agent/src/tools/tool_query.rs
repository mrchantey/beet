//! Tool exchange and query utilities.
//!
//! This module provides [`tool_exchange`] for creating tool sets and
//! [`ToolQuery`] for querying and executing tools from agents.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::AgentQuery;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// Creates a tool exchange that can be used as a tool set for an agent.
///
/// This is similar to [`router_exchange`] but designed for tools:
/// - Constructs the `EndpointTree` immediately on spawn
/// - Can be attached to an agent via the `Tools` relationship
///
/// ## Example
///
/// ```ignore
/// use beet_agent::prelude::*;
/// use beet_router::prelude::*;
/// use beet_flow::prelude::*;
///
/// #[derive(Reflect)]
/// struct AddRequest { a: i32, b: i32 }
///
/// #[derive(Reflect)]
/// struct AddResponse { result: i32 }
///
/// world.spawn((
///     Name::new("MyAgent"),
///     related![Tools,
///         tool_exchange((Sequence, children![
///             EndpointBuilder::post()
///                 .with_path("add")
///                 .with_request_body(BodyMeta::json::<AddRequest>())
///                 .with_response_body(BodyMeta::json::<AddResponse>())
///                 .with_description("Add two numbers")
///                 .with_action(|Json(req): Json<AddRequest>| {
///                     Json(AddResponse { result: req.a + req.b })
///                 }),
///         ]))
///     ],
/// ));
/// ```
pub fn tool_exchange(func: impl BundleFunc) -> impl Bundle {
	router_exchange(func)
}

/// System parameter for querying and executing tools available to an agent.
#[derive(SystemParam)]
pub struct ToolQuery<'w, 's> {
	/// Query for the Tools relationship on agents
	agents: AgentQuery<'w, 's>,
	tools: Query<'w, 's, &'static Tools>,
	parents: Query<'w, 's, &'static ChildOf>,
	/// Query for EndpointTree on tool entities
	endpoint_trees: Query<'w, 's, &'static EndpointTree>,
}

impl ToolQuery<'_, '_> {
	/// Creates a tool name by combining the entity bits with the path.
	/// Format: `tool_{entity_bits}_{path_with_underscores}`
	pub fn tool_name(entity: Entity, path: &str) -> String {
		// unique as long as entity is not despawned
		// TODO to_bits better?
		let index = entity.index_u32();

		let sanitized_path = path
			.trim_start_matches('/')
			.replace('/', "_")
			.replace(':', "")
			.replace('*', "");
		if sanitized_path.is_empty() {
			format!("tool_{index}")
		} else {
			format!("tool_{index}_{sanitized_path}")
		}
	}

	/// Parses a tool name back into entity and path components.
	/// Returns `None` if the name doesn't match the expected format.
	pub fn parse_tool_name(
		world: &World,
		name: &str,
	) -> Option<(Entity, String)> {
		let name = name.strip_prefix("tool_")?;
		let underscore_pos = name.find('_');

		let (index_str, path) = match underscore_pos {
			Some(pos) => {
				let (index, rest) = name.split_at(pos);
				(index, rest.trim_start_matches('_').replace('_', "/"))
			}
			None => (name, String::new()),
		};

		// TODO bits better?
		let index = index_str.parse().ok()?;

		let index = bevy_ecs::entity::EntityIndex::new(index);
		let entity = world.entities().resolve_from_index(index);
		Some((entity, format!("/{path}")))
	}

	/// Collects all tool definitions for the given action entity, its ancestors
	/// and the agent.
	///
	/// Returns a list of `FunctionToolParam` representing all tools available to the agent.
	/// Tool names are prefixed with the tool set entity ID to ensure uniqueness.
	pub fn collect_tools(
		&self,
		action: Entity,
	) -> Result<Vec<openresponses::FunctionToolParam>> {
		let mut tools = Vec::new();

		let mut visited = HashSet::<Entity>::default();
		let agent = self.agents.entity(action);
		for tool in std::iter::once(agent)
			.chain(self.parents.iter_ancestors_inclusive(action))
			.filter_map(|entity| self.tools.get(entity).ok())
			.flat_map(|tools| tools.iter())
		{
			if visited.contains(&tool) {
				continue;
			}
			let tree = self.endpoint_trees.get(tool)?;
			self.collect_tools_from_tree(tree, tool, &mut tools);
			visited.insert(tool);
		}

		tools.xok()
	}

	/// Recursively collects tool params from an endpoint tree.
	fn collect_tools_from_tree(
		&self,
		tree: &EndpointTree,
		tool_set_entity: Entity,
		tools: &mut Vec<openresponses::FunctionToolParam>,
	) {
		if let Some(endpoint) = &tree.endpoint {
			let tool = Self::endpoint_to_function_tool_param(
				endpoint,
				tool_set_entity,
			);
			tools.push(tool);
		}

		for child in &tree.children {
			self.collect_tools_from_tree(child, tool_set_entity, tools);
		}
	}

	/// Converts an endpoint to a FunctionToolParam for the model API.
	fn endpoint_to_function_tool_param(
		endpoint: &Endpoint,
		tool_set_entity: Entity,
	) -> openresponses::FunctionToolParam {
		let path = endpoint.path().annotated_route_path().to_string();
		let name = Self::tool_name(tool_set_entity, &path);

		let mut param = openresponses::FunctionToolParam::new(&name);

		if let Some(desc) = endpoint.description() {
			param = param.with_description(desc);
		}

		if let Some(schema) = endpoint.request_body().schema() {
			let json_schema = body_meta_to_json_schema(schema);
			param = param.with_parameters(json_schema);
		}

		param
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
	obj.insert(
		"type".to_string(),
		serde_json::Value::String("object".to_string()),
	);
	obj.insert(
		"properties".to_string(),
		serde_json::Value::Object(properties),
	);

	if !required.is_empty() {
		obj.insert("required".to_string(), serde_json::Value::Array(required));
	}

	serde_json::Value::Object(obj)
}

/// Converts a `FieldSchema` to a JSON Schema property.
fn field_to_json_schema(field: &FieldSchema) -> serde_json::Value {
	let type_path = field.type_path();

	// Map common Rust types to JSON Schema types
	let json_type = if type_path.contains("String") || type_path.contains("str")
	{
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
	use beet_flow::prelude::*;

	#[derive(Reflect)]
	struct TestRequest {
		value: String,
	}

	#[derive(Reflect)]
	struct TestResponse {
		result: String,
	}

	#[test]
	fn tool_name_generation() {
		let entity = Entity::from_bits(42);
		let name = ToolQuery::tool_name(entity, "/add");
		assert!(name.starts_with("tool_"));
		assert!(name.contains("add"));
	}

	#[test]
	fn tool_name_roundtrip() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let original_path = "/users/create";
		let name = ToolQuery::tool_name(entity, original_path);

		let (parsed_entity, parsed_path) =
			ToolQuery::parse_tool_name(&world, &name).unwrap();

		// Entity bits should match
		assert_eq!(entity.to_bits(), parsed_entity.to_bits());
		// Path should be reconstructed (may differ slightly due to sanitization)
		assert!(parsed_path.contains("users"));
		assert!(parsed_path.contains("create"));
	}

	#[test]
	fn tool_name_empty_path() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let name = ToolQuery::tool_name(entity, "/");
		let (parsed_entity, _) =
			ToolQuery::parse_tool_name(&world, &name).unwrap();
		assert_eq!(entity.to_bits(), parsed_entity.to_bits());
	}

	#[beet_core::test]
	async fn tool_exchange_creates_endpoint_tree() {
		let mut world = RouterPlugin::world();
		let entity = world.spawn(tool_exchange(|| {
			(Sequence, children![
				EndpointBuilder::post()
					.with_path("test")
					.with_description("A test tool")
					.with_request_body(BodyMeta::json::<TestRequest>())
					.with_action(|| "ok"),
			])
		}));

		// EndpointTree should be present
		entity.get::<EndpointTree>().is_some().xpect_true();

		let tree = entity.get::<EndpointTree>().unwrap();
		tree.flatten().len().xpect_eq(1);
	}
}
