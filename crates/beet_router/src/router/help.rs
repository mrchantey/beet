//! Help middleware that renders route documentation using beet_node.
//!
//! When the `--help` param is present, renders a scene entity tree
//! describing available routes, then converts it to a response
//! via the scene rendering pipeline. If the param is absent,
//! calls the inner handler via [`Next`].

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;
use beet_tool::prelude::*;

/// Middleware that intercepts `--help` and renders scoped help
/// as a beet_node scene entity tree.
#[tool]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn HelpHandler(
	cx: ToolContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let (request, next) = cx.take();

	if !request.has_param("help") {
		return next.call(request).await;
	}

	let path = request.path().clone();
	let parts = request.parts().clone();

	let nodes = caller
		.clone()
		.with_state::<AncestorQuery<&RouteTree>, Result<_>>(
			move |entity, query| {
				let tree = query.get(entity)?;
				let nodes = if let Some(subtree) = tree.find_subtree(&path) {
					subtree.flatten_nodes()
				} else {
					tree.flatten_nodes()
				};
				let filtered: Vec<&ToolNode> = nodes
					.into_iter()
					.filter(|node| {
						node.path.annotated_rel_path().last_segment()
							!= Some("help")
					})
					.collect();
				filtered
					.into_iter()
					.cloned()
					.collect::<Vec<ToolNode>>()
					.xok()
			},
		)
		.await?;

	let scene = spawn_help_scene(&caller, &nodes).await;
	let response =
		SceneToolRenderer::render_entity(&caller, scene, parts).await?;
	Ok(response)
}

/// Fallback handler that shows help scoped to the nearest ancestor scene
/// of an unmatched path. Returns a NOT_FOUND status with the help scene.
#[tool]
pub(crate) async fn ContextualNotFound(
	cx: ToolContext<Request>,
) -> Result<Response> {
	let path = cx.input.path().clone();

	let (preamble, nodes) = cx
		.caller
		.with_state::<AncestorQuery<&RouteTree>, Result<_>>(
			move |entity, query| {
				let tree = query.get(entity)?;
				let (preamble, help_nodes) =
					nearest_ancestor_help_nodes(tree, &path);
				(preamble, help_nodes).xok()
			},
		)
		.await?;

	let scene = spawn_not_found_scene(&cx.caller, &preamble, &nodes).await;
	let mut response = SceneToolRenderer::render_entity(
		&cx.caller,
		scene,
		cx.input.parts().clone(),
	)
	.await?;
	response.parts.status = StatusCode::NOT_FOUND;
	Ok(response)
}

/// Walks path segments from longest to shortest prefix, returning
/// help nodes for the first ancestor that matches a scene.
fn nearest_ancestor_help_nodes(
	tree: &RouteTree,
	segments: &[String],
) -> (String, Vec<ToolNode>) {
	for length in (1..segments.len()).rev() {
		let prefix = &segments[..length];
		if let Some(node) = tree.find(prefix) {
			if node.is_scene {
				let prefix_str = prefix.join("/");
				let preamble = format!(
					"Route /{} not found. Showing help for /{}:",
					segments.join("/"),
					prefix_str,
				);
				let help_tree = tree.find_subtree(prefix).unwrap_or(tree);
				let nodes = filtered_nodes(help_tree);
				return (preamble, nodes);
			}
		}
	}
	let preamble = format!("Route /{} not found.", segments.join("/"));
	let nodes = filtered_nodes(tree);
	(preamble, nodes)
}

fn filtered_nodes(tree: &RouteTree) -> Vec<ToolNode> {
	tree.flatten_nodes()
		.into_iter()
		.filter(|node| {
			node.path.annotated_rel_path().last_segment() != Some("help")
		})
		.cloned()
		.collect()
}

/// Spawns a help scene entity tree with route documentation.
async fn spawn_help_scene(caller: &AsyncEntity, nodes: &[ToolNode]) -> Entity {
	let children: Vec<(Element, OnSpawn)> = nodes
		.iter()
		.map(|node| format_tool_node_bundle(node))
		.collect();

	let world = caller.world();
	let entity = world
		.spawn_then((
			DespawnOnRender,
			Element::new("div"),
			OnSpawn::insert_child(
				Element::new("h2").with_inner_text("Available routes"),
			),
		))
		.await;
	for child in children {
		entity.insert_then(OnSpawn::insert_child(child)).await;
	}
	entity.id()
}

/// Spawns a not-found scene entity tree with a preamble and help.
async fn spawn_not_found_scene(
	caller: &AsyncEntity,
	preamble: &str,
	nodes: &[ToolNode],
) -> Entity {
	let children: Vec<(Element, OnSpawn)> = nodes
		.iter()
		.map(|node| format_tool_node_bundle(node))
		.collect();

	let world = caller.world();
	let entity = world
		.spawn_then((
			DespawnOnRender,
			Element::new("div"),
			OnSpawn::insert_child(Element::new("p").with_inner_text(preamble)),
			OnSpawn::insert_child(
				Element::new("h2").with_inner_text("Available routes"),
			),
		))
		.await;
	for child in children {
		entity.insert_then(OnSpawn::insert_child(child)).await;
	}
	entity.id()
}

/// Creates an element bundle describing a single route node.
///
/// Each route renders as a `<li>` with clearly separated sections:
/// path, kind tag, description, type signature, and parameters.
fn format_tool_node_bundle(node: &ToolNode) -> (Element, OnSpawn) {
	let path = node.path.annotated_rel_path().to_string();

	// path with leading slash and kind tag
	let heading = if node.is_scene {
		format!("/{path} [scene]")
	} else if let Some(method) = &node.method {
		format!("/{path} [{method}]")
	} else {
		format!("/{path}")
	};

	let mut text = heading;

	// description on same line after em dash
	if let Some(description) = &node.description {
		text.push_str(&format!(" — {}", description.as_str()));
	}

	// input/output types — skip trivial, exchange, and scene signatures
	let input_type = node.meta.input().type_name();
	let output_type = node.meta.output().type_name();
	let is_trivial = input_type == "()" && output_type == "()";
	let is_exchange =
		input_type.ends_with("Request") && output_type.ends_with("Response");
	if !is_trivial && !is_exchange && !node.is_scene {
		text.push_str(&format!(" ({input_type} → {output_type})"));
	}

	// parameters
	for param in node.params.iter() {
		text.push_str(&format!(" {param}"));
	}

	(
		Element::new("li"),
		OnSpawn::insert_child(Value::Str(text.into())),
	)
}

/// Format a [`RouteTree`] as a help string, listing both scenes and tools.
///
/// The help tool itself is excluded from the listing.
/// Retained for backward compatibility with tests and plaintext rendering.
pub fn format_route_help(tree: &RouteTree) -> String {
	let mut output = String::new();
	output.push_str("Available routes:\n\n");

	let nodes = tree.flatten_nodes();

	let filtered: Vec<&ToolNode> = nodes
		.into_iter()
		.filter(|node| {
			node.path.annotated_rel_path().last_segment() != Some("help")
		})
		.collect();

	if filtered.is_empty() {
		output.push_str("  (none)\n");
		return output;
	}

	for node in filtered {
		format_tool_node_text(&mut output, node);
	}

	output
}

/// Format a [`ToolNode`] as plaintext for CLI output.
fn format_tool_node_text(output: &mut String, node: &ToolNode) {
	let path = node.path.annotated_rel_path();

	if node.is_scene {
		output.push_str(&format!("  /{} [scene]\n", path));
	} else {
		output.push_str(&format!("  /{}", path));
		if let Some(method) = &node.method {
			output.push_str(&format!(" [{}]", method));
		}
		output.push('\n');

		if let Some(description) = &node.description {
			output.push_str(&format!("    {}\n", description.as_str()));
		}

		let input_type = node.meta.input().type_name();
		let output_type = node.meta.output().type_name();
		// Skip Request→Response and scene tool signatures
		let is_exchange = input_type.ends_with("Request")
			&& output_type.ends_with("Response");
		if !is_exchange && !node.is_scene {
			if input_type != "()" {
				output.push_str(&format!("    input:  {}\n", input_type));
			}
			if output_type != "()" {
				output.push_str(&format!("    output: {}\n", output_type));
			}
		}
	}

	for param in node.params.iter() {
		output.push_str(&format!("    {}\n", param));
	}

	output.push('\n');
}


#[cfg(test)]
mod test {
	use super::*;

	/// Adds help as a tool located at `/help`.
	fn help() -> impl Bundle {
		(
			PathPartial::new("help"),
			Tool::<(), String>::new_system(help_system),
		)
	}
	fn help_system(
		In(cx): In<ToolContext>,
		ancestors: Query<&ChildOf>,
		trees: Query<&RouteTree>,
	) -> Result<String> {
		let root = ancestors.root_ancestor(cx.id());
		let tree = trees.get(root).map_err(|_| {
			bevyhow!("No RouteTree found on root ancestor, cannot render help")
		})?;
		format_route_help(tree).xok()
	}

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	#[beet_core::test]
	async fn help_lists_tools() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				help(),
				increment(FieldRef::new("count")),
				decrement(FieldRef::new("count")),
			])
			.flush();

		let help_entity = world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["help"])
			.unwrap()
			.entity;

		let output = world
			.entity_mut(help_entity)
			.call::<(), String>(())
			.await
			.unwrap();

		output.contains("Available routes").xpect_true();
		output.contains("increment").xpect_true();
		output.contains("decrement").xpect_true();
		// help itself should be excluded from the listing
		output.contains("help").xpect_false();
	}

	#[beet_core::test]
	async fn help_shows_nested_tools() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				help(),
				(PathPartial::new("counter"), children![increment(
					FieldRef::new("count")
				),]),
			])
			.flush();

		let help_entity = world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["help"])
			.unwrap()
			.entity;

		let output = world
			.entity_mut(help_entity)
			.call::<(), String>(())
			.await
			.unwrap();

		output.contains("counter/increment").xpect_true();
	}

	#[beet_core::test]
	async fn help_shows_input_output_types() {
		let mut world = router_world();
		let root = world
			.spawn(children![help(), add(FieldRef::new("value")),])
			.flush();

		let help_entity = world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["help"])
			.unwrap()
			.entity;

		let output = world
			.entity_mut(help_entity)
			.call::<(), String>(())
			.await
			.unwrap();

		// add takes i64 input and returns i64
		output.contains("i64").xpect_true();
	}

	#[beet_core::test]
	async fn help_with_no_other_tools() {
		let mut world = router_world();
		let root = world.spawn(children![help()]).flush();

		let help_entity = world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["help"])
			.unwrap()
			.entity;

		let output = world
			.entity_mut(help_entity)
			.call::<(), String>(())
			.await
			.unwrap();

		output.contains("(none)").xpect_true();
		output.contains("Available routes").xpect_true();
	}

	#[beet_core::test]
	async fn help_includes_scenes() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				help(),
				fixed_scene("about", || Element::new("p")
					.with_inner_text("about")),
				increment(FieldRef::new("count")),
			])
			.flush();

		let help_entity = world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["help"])
			.unwrap()
			.entity;

		let output = world
			.entity_mut(help_entity)
			.call::<(), String>(())
			.await
			.unwrap();

		// scenes should appear with a [scene] marker
		output.contains("about").xpect_true();
		output.contains("[scene]").xpect_true();
		// tools should still appear
		output.contains("increment").xpect_true();
	}
}
