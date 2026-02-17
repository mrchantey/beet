//! A tool that renders help documentation for registered routes.
//!
//! Traverses to the root ancestor, reads the [`RouteTree`], and formats
//! it as a human-readable help string showing cards and tools.

use crate::prelude::*;
use beet_core::prelude::*;

/// Checks for the `--help` param and renders scoped help text.
pub(crate) async fn help_handler(
	cx: AsyncToolContext<Request>,
) -> Result<Outcome<Response, Request>> {
	if cx.has_param("help") {
		let path = cx.input.path().clone();
		let tool_entity = cx.tool.id();
		let help_text = cx
			.tool
			.world()
			.with_then(move |world: &mut World| -> Result<String> {
				let tree = root_route_tree(world, tool_entity)?;
				// Scope help to the requested path prefix
				if path.is_empty() {
					format_route_help(tree).xok()
				} else if let Some(subtree) = tree.find_subtree(&path) {
					format_route_help(subtree).xok()
				} else {
					nearest_ancestor_help(tree, &path).xok()
				}
			})
			.await?;

		Outcome::Pass(Response::ok_body(help_text, "text/plain")).xok()
	} else {
		Fail(cx.input).xok()
	}
}

/// Fallback handler that shows help scoped to the nearest ancestor card
/// of an unmatched path.
pub(crate) async fn contextual_not_found_handler(
	cx: AsyncToolContext<Request>,
) -> Result<Outcome<Response, Request>> {
	let path = cx.input.path().clone();
	let tool_entity = cx.tool.id();
	let help_text = cx
		.tool
		.world()
		.with_then(move |world: &mut World| -> Result<String> {
			let tree = root_route_tree(world, tool_entity)?;
			nearest_ancestor_help(tree, &path).xok()
		})
		.await?;

	Outcome::Pass(Response::from_status_body(
		StatusCode::NotFound,
		help_text,
		"text/plain",
	))
	.xok()
}

/// Walks the path segments from longest to shortest prefix, returning
/// help for the first ancestor that matches a card. Falls back to
/// root help if nothing matches.
fn nearest_ancestor_help(tree: &RouteTree, segments: &Vec<String>) -> String {
	// Try progressively shorter prefixes
	for length in (1..segments.len()).rev() {
		let prefix = &segments[..length];
		if let Some(node) = tree.find(prefix) {
			if node.is_card {
				let prefix_str = prefix.join("/");
				let mut output = String::new();
				output.push_str(&format!(
					"Route /{} not found. Showing help for /{}:\n\n",
					segments.join("/"),
					prefix_str,
				));
				// Scope help to the matching ancestor subtree
				let help_tree = tree.find_subtree(prefix).unwrap_or(tree);
				output.push_str(&format_route_help(help_tree));
				return output;
			}
		}
	}

	// Nothing matched at all - show root help with a not-found preamble
	let mut output = String::new();
	output.push_str(&format!("Route /{} not found.\n\n", segments.join("/"),));
	output.push_str(&format_route_help(tree));
	output
}

/// Format a [`RouteTree`] as a help string, listing both cards and tools.
///
/// The help tool itself is excluded from the listing.
/// This is the primary entry point for rendering help text and is
/// reused by the interface tool for contextual help. Can be called on
/// a subtree from [`RouteTree::find_subtree`] to scope output to a
/// specific path prefix.
pub fn format_route_help(tree: &RouteTree) -> String {
	let mut output = String::new();
	output.push_str("Available routes:\n\n");

	let nodes = tree.flatten_nodes();

	let filtered: Vec<&ToolNode> = nodes
		.into_iter()
		.filter(|node| {
			!node
				.path
				.annotated_route_path()
				.to_string()
				.ends_with("/help")
		})
		.collect();

	if filtered.is_empty() {
		output.push_str("  (none)\n");
		return output;
	}

	for node in filtered {
		format_tool_node(&mut output, node);
	}

	output
}

/// Format a [`ToolNode`] into the output string, displaying `[card]`
/// for card routes and input/output types for regular tools.
fn format_tool_node(output: &mut String, node: &ToolNode) {
	let path = node.path.annotated_route_path();

	if node.is_card {
		output.push_str(&format!("  {} [card]\n", path));
	} else {
		output.push_str(&format!("  {}", path));
		if let Some(method) = &node.method {
			output.push_str(&format!(" [{}]", method));
		}
		output.push('\n');

		// input/output types, skip trivial `()` types
		let input_type = node.meta.input().type_name();
		let output_type = node.meta.output().type_name();
		if input_type != "()" {
			output.push_str(&format!("    input:  {}\n", input_type));
		}
		if output_type != "()" {
			output.push_str(&format!("    output: {}\n", output_type));
		}
	}

	// params
	for param in node.params.iter() {
		output.push_str(&format!("    {}\n", param));
	}

	output.push('\n');
}


#[cfg(test)]
mod test {
	use super::*;

	/// Adds help as a tool located at `/help`.
	/// Usually this is handled as an interface tool, added via ?help.
	fn help() -> impl Bundle { (PathPartial::new("help"), tool(help_system)) }
	fn help_system(
		cx: In<ToolContext>,
		ancestors: Query<&ChildOf>,
		trees: Query<&RouteTree>,
	) -> Result<String> {
		let root = ancestors.root_ancestor(cx.tool);
		let tree = trees.get(root).map_err(|_| {
			bevyhow!("No RouteTree found on root ancestor, cannot render help")
		})?;
		format_route_help(tree).xok()
	}

	#[test]
	fn help_lists_tools() {
		let mut world = StackPlugin::world();
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
			.call_blocking::<(), String>(())
			.unwrap();

		output.contains("Available routes").xpect_true();
		output.contains("increment").xpect_true();
		output.contains("decrement").xpect_true();
		// help itself should be excluded from the listing
		output.contains("help").xpect_false();
	}

	#[test]
	fn help_shows_nested_tools() {
		let mut world = StackPlugin::world();
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
			.call_blocking::<(), String>(())
			.unwrap();

		output.contains("counter/increment").xpect_true();
	}

	#[test]
	fn help_shows_input_output_types() {
		let mut world = StackPlugin::world();
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
			.call_blocking::<(), String>(())
			.unwrap();

		// add takes i64 input and returns i64
		output.contains("i64").xpect_true();
	}

	#[test]
	fn help_with_no_other_tools() {
		let mut world = StackPlugin::world();
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
			.call_blocking::<(), String>(())
			.unwrap();

		output.contains("(none)").xpect_true();
		output.contains("Available routes").xpect_true();
	}

	#[test]
	fn help_includes_cards() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((default_router(), children![
				help(),
				card("about", || Paragraph::with_text("about")),
				increment(FieldRef::new("count")),
			]))
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
			.call_blocking::<(), String>(())
			.unwrap();

		// cards should appear with a [card] marker
		output.contains("about").xpect_true();
		output.contains("[card]").xpect_true();
		// tools should still appear
		output.contains("increment").xpect_true();
	}

	#[beet_core::test]
	async fn default_router_renders_help() {
		let mut world = StackPlugin::world();

		let root = world
			.spawn((default_router(), children![
				markdown_render_tool(),
				increment(FieldRef::new("count")),
				card("about", || Paragraph::with_text("about")),
			]))
			.flush();

		let body = world
			.entity_mut(root)
			.call::<Request, Response>(Request::from_cli_str("--help").unwrap())
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("Available routes").xpect_true();
		body.contains("increment").xpect_true();
		body.contains("about").xpect_true();
	}

	#[beet_core::test]
	async fn help_scoped_to_prefix() {
		let body = StackPlugin::world()
			.spawn((default_router(), children![
				markdown_render_tool(),
				(
					card("counter", || Paragraph::with_text("counter")),
					children![increment(FieldRef::new("count")),],
				),
				card("about", || Paragraph::with_text("about")),
			]))
			.call::<Request, Response>(
				Request::from_cli_str("counter --help").unwrap(),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await;
		// Should show routes under counter
		body.contains("increment").xpect_true();
		// Should not show sibling routes
		body.contains("about").xpect_false();
	}

	#[beet_core::test]
	async fn not_found_shows_ancestor_help() {
		StackPlugin::world()
			.spawn((default_router(), children![
				markdown_render_tool(),
				increment(FieldRef::new("count")),
			]))
			.call::<Request, Response>(
				Request::from_cli_str("nonexistent").unwrap(),
			)
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_contains("not found")
			.xpect_contains("Available routes");
	}

	#[beet_core::test]
	async fn not_found_shows_scoped_ancestor_help() {
		StackPlugin::world()
			.spawn((default_router(), children![
				markdown_render_tool(),
				(
					card("counter", || Paragraph::with_text("counter")),
					children![increment(FieldRef::new("count")),],
				),
				card("about", || Paragraph::with_text("about")),
			]))
			.call::<Request, Response>(
				Request::from_cli_str("counter nonsense").unwrap(),
			)
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_contains("not found")
			.xpect_contains("increment")
			.xnot()
			.xpect_contains("about");
	}
}
