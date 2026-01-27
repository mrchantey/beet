//! Tool exchange and query utilities.
//!
//! This module provides [`tool_exchange`] for creating tool sets and
//! [`ToolQuery`] for querying tools from agents.

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

/// System parameter for querying tools available to an agent.
#[derive(SystemParam)]
pub struct ToolQuery<'w, 's> {
	/// Query for the Tools relationship on agents
	pub tools: AgentQuery<'w, 's, &'static Tools>,
	/// Query for EndpointTree on tool entities
	pub endpoint_trees: Query<'w, 's, &'static EndpointTree>,
}

impl ToolQuery<'_, '_> {
	/// Collects all tool definitions for the given action entity.
	///
	/// Returns a list of `ToolMeta` representing all tools available to the agent.
	/// Tool names are prefixed with the tool set entity ID to ensure uniqueness.
	pub fn collect_tools(
		&self,
		action: Entity,
	) -> Result<Vec<openresponses::FunctionToolParam>> {
		let mut tools = Vec::new();

		if let Ok(tool_sets) = self.tools.get(action) {
			for tool_set_entity in tool_sets.iter() {
				let tree = self.endpoint_trees.get(tool_set_entity)?;
				self.collect_tools_from_tree(tree, tool_set_entity, &mut tools);
			}
		}

		tools.xok()
	}

	/// Recursively collects tool metas from an endpoint tree.
	fn collect_tools_from_tree(
		&self,
		tree: &EndpointTree,
		tool_set_entity: Entity,
		tools: &mut Vec<openresponses::FunctionToolParam>,
	) {
		if let Some(endpoint) = &tree.endpoint {
			tools.push(
				ToolMeta::from_endpoint(endpoint, tool_set_entity)
					.to_function_tool_param(),
			);
		}

		for child in &tree.children {
			self.collect_tools_from_tree(child, tool_set_entity, tools);
		}
	}

	/// Finds a tool set entity by parsing a tool name.
	///
	/// Returns the tool set entity and the path within that tool set.
	pub fn find_tool_set(&self, tool_name: &str) -> Option<(Entity, String)> {
		ToolMeta::parse_tool_name(tool_name)
	}
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
