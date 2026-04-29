//! Help middleware that renders route documentation using beet_node.
//!
//! When the `--help` param is present, renders a scene entity tree
//! describing available routes, then converts it to a response
//! via the scene rendering pipeline. If the param is absent,
//! calls the inner handler via [`Next`].

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;

/// Middleware that intercepts `--help` and renders scoped help
/// as a beet_node scene entity tree.
#[action]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn HelpHandler(
	cx: ActionContext<(Request, Next<Request, Response>)>,
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
				let filtered: Vec<&ActionNode> = nodes
					.into_iter()
					.filter(|node| {
						node.path.annotated_rel_path().last_segment()
							!= Some("help")
					})
					.collect();
				filtered
					.into_iter()
					.cloned()
					.collect::<Vec<ActionNode>>()
					.xok()
			},
		)
		.await?;

	let scene = spawn_help_scene(&caller, &nodes).await;
	scene.render(&caller, parts).await
}

/// Fallback handler that shows help scoped to the nearest ancestor scene
/// of an unmatched path. Returns a NOT_FOUND status with the help scene.
#[action]
pub(crate) async fn ContextualNotFound(
	cx: ActionContext<Request>,
) -> Result<Response> {
	let path = cx.input.path().clone();

	let (info, nodes) = cx
		.caller
		.with_state::<AncestorQuery<&RouteTree>, Result<_>>(
			move |entity, query| {
				let tree = query.get(entity)?;
				let (info, help_nodes) =
					nearest_ancestor_help_nodes(tree, &path);
				(info, help_nodes).xok()
			},
		)
		.await?;

	let scene = spawn_not_found_scene(&cx.caller, info, &nodes).await;
	let mut response =
		scene.render(&cx.caller, cx.input.parts().clone()).await?;
	response.parts.status = StatusCode::NOT_FOUND;
	Ok(response)
}

/// Data describing a not-found route for rendering the preamble.
struct NotFoundInfo {
	/// The path that was not found.
	not_found_path: String,
	/// The nearest ancestor scene path, if any.
	ancestor_path: Option<String>,
}

/// Walks path segments from longest to shortest prefix, returning
/// help nodes for the first ancestor that matches a scene.
fn nearest_ancestor_help_nodes(
	tree: &RouteTree,
	segments: &[String],
) -> (NotFoundInfo, Vec<ActionNode>) {
	let not_found_path = segments.join("/");

	for length in (1..segments.len()).rev() {
		let prefix = &segments[..length];
		if let Some(node) = tree.find(prefix) {
			if node.is_scene() {
				let prefix_str = prefix.join("/");
				let help_tree = tree.find_subtree(prefix).unwrap_or(tree);
				let nodes = filtered_nodes(help_tree);
				return (
					NotFoundInfo {
						not_found_path,
						ancestor_path: Some(prefix_str),
					},
					nodes,
				);
			}
		}
	}
	let nodes = filtered_nodes(tree);
	(
		NotFoundInfo {
			not_found_path,
			ancestor_path: None,
		},
		nodes,
	)
}

fn filtered_nodes(tree: &RouteTree) -> Vec<ActionNode> {
	tree.flatten_nodes()
		.into_iter()
		.filter(|node| {
			node.path.annotated_rel_path().last_segment() != Some("help")
		})
		.cloned()
		.collect()
}
/// Spawns a help scene entity tree with route documentation.
async fn spawn_help_scene(
	caller: &AsyncEntity,
	nodes: &[ActionNode],
) -> SceneEntity {
	let children: Vec<OnSpawn> = nodes
		.iter()
		.map(|node| format_action_node_bundle(node).any_bundle())
		.collect();

	let world = caller.world();
	// spawn children first
	let mut child_ids = Vec::new();
	for child in children {
		let child_entity = world.spawn_then(child).await;
		child_ids.push(child_entity.id());
	}
	// build parent with heading
	let entity = world
		.spawn_then(rsx! {
			<div>
				<h2>"Available routes"</h2>
			</div>
		})
		.await;
	// add pre-spawned children to parent
	let parent_id = entity.id();
	entity
		.with_then(move |mut entity| {
			entity.world_scope(move |world| {
				for child_id in child_ids {
					world.entity_mut(parent_id).add_child(child_id);
				}
			});
		})
		.await;
	SceneEntity::new_ephemeral(entity.id())
}

/// Builds the not-found preamble with anchor tags for the missing route
/// and optional ancestor route.
fn not_found_preamble(info: NotFoundInfo) -> OnSpawn {
	let not_found_path = info.not_found_path;
	let not_found_href = format!("/{not_found_path}");

	if let Some(ancestor) = info.ancestor_path {
		let ancestor_href = format!("/{ancestor}");
		rsx! {
			<div>
				<p>
					"Route "
					<a href=not_found_href.clone()>{not_found_href}</a>
					" not found. Showing help for "
					<a href=ancestor_href.clone()>{ancestor_href}</a>
					":"
				</p>
				<h2>"Available routes"</h2>
			</div>
		}
		.any_bundle()
	} else {
		rsx! {
			<div>
				<p>
					"Route "
					<a href=not_found_href.clone()>{not_found_href}</a>
					" not found."
				</p>
				<h2>"Available routes"</h2>
			</div>
		}
		.any_bundle()
	}
}

/// Spawns a not-found scene entity tree with anchor-tagged preamble and help.
async fn spawn_not_found_scene(
	caller: &AsyncEntity,
	info: NotFoundInfo,
	nodes: &[ActionNode],
) -> SceneEntity {
	let children: Vec<OnSpawn> = nodes
		.iter()
		.map(|node| format_action_node_bundle(node).any_bundle())
		.collect();

	let world = caller.world();
	// spawn children first
	let mut child_ids = Vec::new();
	for child in children {
		let child_entity = world.spawn_then(child).await;
		child_ids.push(child_entity.id());
	}
	// build parent from preamble
	let entity = world.spawn_then(not_found_preamble(info)).await;
	// add pre-spawned children to parent
	let parent_id = entity.id();
	entity
		.with_then(move |mut entity| {
			entity.world_scope(move |world| {
				for child_id in child_ids {
					world.entity_mut(parent_id).add_child(child_id);
				}
			});
		})
		.await;
	SceneEntity::new_ephemeral(entity.id())
}

/// Creates an element bundle describing a single route node.
///
/// Each route renders as a `<li>` containing the path heading
/// and a nested `<ul>` with description, type info, and parameters.
fn format_action_node_bundle(node: &ActionNode) -> (Element, OnSpawn) {
	let path = node.path.annotated_rel_path().to_string();

	// path with leading slash and kind tag
	let heading = if node.is_scene() {
		format!("/{path} [scene]")
	} else if let Some(method) = &node.method {
		format!("/{path} [{method}]")
	} else {
		format!("/{path}")
	};

	// collect detail items as (label, value) pairs
	let mut details: Vec<(String, String)> = Vec::new();

	if let Some(description) = node.description() {
		details.push(("description".into(), description.to_string()));
	}

	let input_type = node.meta.input().type_name();
	let output_type = node.meta.output().type_name();
	let is_trivial = input_type == "()" && output_type == "()";
	let is_exchange =
		input_type.ends_with("Request") && output_type.ends_with("Response");
	if !is_trivial && !is_exchange && !node.is_scene() {
		details.push(("input".into(), input_type.to_string()));
		details.push(("output".into(), output_type.to_string()));
	}

	for param in node.params.iter() {
		details.push(("param".into(), param.to_string()));
	}

	(
		Element::new("li"),
		OnSpawn::new(move |entity| {
			let li_id = entity.id();
			entity.world_scope(move |world| {
				// spawn heading text first
				let heading_entity =
					world.spawn(Value::Str(heading.into())).flush();
				// spawn nested detail list if needed
				if !details.is_empty() {
					let lis: Vec<Entity> = details
						.into_iter()
						.map(|(label, value)| {
							world
								.spawn(rsx! {
									<li>
										<strong>{format!("{label}:")}</strong>
										{format!(" {value}")}
									</li>
								})
								.flush()
						})
						.collect();
					let ul = world.spawn(rsx! { <ul>{lis}</ul> }).flush();
					// add heading and ul as children of li
					world.entity_mut(li_id).add_children(&[heading_entity, ul]);
				} else {
					world.entity_mut(li_id).add_child(heading_entity);
				}
			});
		}),
	)
}

/// Format a [`RouteTree`] as a help string, listing both scenes and actions.
///
/// The help action itself is excluded from the listing.
/// Retained for backward compatibility with tests and plaintext rendering.
pub fn format_route_help(tree: &RouteTree) -> String {
	let mut output = String::new();
	output.push_str("Available routes:\n\n");

	let nodes = tree.flatten_nodes();

	let filtered: Vec<&ActionNode> = nodes
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
		format_action_node_text(&mut output, node);
	}

	output
}

/// Format an [`ActionNode`] as plaintext for CLI output.
fn format_action_node_text(output: &mut String, node: &ActionNode) {
	let path = node.path.annotated_rel_path();

	if node.is_scene() {
		output.push_str(&format!("  /{} [scene]\n", path));
	} else {
		output.push_str(&format!("  /{}", path));
		if let Some(method) = &node.method {
			output.push_str(&format!(" [{}]", method));
		}
		output.push('\n');

		if let Some(description) = node.description() {
			output.push_str(&format!("    {}\n", description));
		}

		let input_type = node.meta.input().type_name();
		let output_type = node.meta.output().type_name();
		// Skip Request->Response and scene action signatures
		let is_exchange = input_type.ends_with("Request")
			&& output_type.ends_with("Response");
		if !is_exchange && !node.is_scene() {
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

	/// Adds help as an action located at `/help`.
	fn help() -> impl Bundle {
		(
			PathPartial::new("help"),
			Action::<(), String>::new_system(help_system),
			ActionMeta::of::<(), (), String>(),
		)
	}
	fn help_system(
		In(cx): In<ActionContext>,
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
	async fn help_lists_actions() {
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
	async fn help_shows_nested_actions() {
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
	async fn help_with_no_other_actions() {
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
				fixed_scene("about", rsx! { <p>"about"</p> }),
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
		// actions should still appear
		output.contains("increment").xpect_true();
	}
}
