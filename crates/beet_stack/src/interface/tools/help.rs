//! A tool that renders help documentation for registered routes.
//!
//! Traverses to the root ancestor, reads the [`RouteTree`], and formats
//! it as a human-readable help string showing cards and tools.

use crate::prelude::*;
use beet_core::prelude::*;

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

	let filtered: Vec<&RouteNode> = nodes
		.into_iter()
		.filter(|node| {
			!node
				.path()
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
		format_route_node(&mut output, node);
	}

	output
}

/// Format a single [`RouteNode`] (card or tool) into the output string.
fn format_route_node(output: &mut String, node: &RouteNode) {
	match node {
		RouteNode::Card(card) => {
			format_card_node(output, card);
		}
		RouteNode::Tool(tool) => {
			format_tool_node(output, tool);
		}
	}
}

/// Format a [`CardNode`] into the output string.
fn format_card_node(output: &mut String, card: &CardNode) {
	let path = card.path.annotated_route_path();
	output.push_str(&format!("  {} [card]\n", path));

	// params
	for param in card.params.iter() {
		output.push_str(&format!("    {}\n", param));
	}

	output.push('\n');
}

/// Format a [`ToolNode`] into the output string.
fn format_tool_node(output: &mut String, node: &ToolNode) {
	let path = node.path.annotated_route_path();
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

	// params
	for param in node.params.iter() {
		output.push_str(&format!("    {}\n", param));
	}

	output.push('\n');
}


#[cfg(test)]
mod test {
	use super::*;

	/// adds help as a tool located at `/help`
	/// usually this is handled as an interface tool, added via ?help
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
			.spawn((Card, children![
				help(),
				increment(FieldRef::new("count")),
				decrement(FieldRef::new("count")),
			]))
			.flush();

		let help_entity = world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["help"])
			.unwrap()
			.entity();

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
			.spawn((Card, children![
				help(),
				(PathPartial::new("counter"), children![increment(
					FieldRef::new("count")
				),]),
			]))
			.flush();

		let help_entity = world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["help"])
			.unwrap()
			.entity();

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
			.spawn((Card, children![help(), add(FieldRef::new("value")),]))
			.flush();

		let help_entity = world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["help"])
			.unwrap()
			.entity();

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
		let root = world.spawn((Card, children![help()])).flush();

		let help_entity = world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["help"])
			.unwrap()
			.entity();

		let output = world
			.entity_mut(help_entity)
			.call_blocking::<(), String>(())
			.unwrap();

		// The root card appears (it has an implicit "/" path),
		// but no tools besides help (which is filtered out)
		output.contains("[card]").xpect_true();
		output.contains("Available routes").xpect_true();
	}

	#[test]
	fn help_includes_cards() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, children![
				help(),
				card("about"),
				increment(FieldRef::new("count")),
			]))
			.flush();

		let help_entity = world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["help"])
			.unwrap()
			.entity();

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
}
