//! A tool that renders help documentation for all registered tools.
//!
//! Traverses to the root ancestor, reads the [`ToolTree`], and formats
//! it as a human-readable help string.

use crate::prelude::*;
use beet_core::prelude::*;

/// Creates a help tool accessible at `/help`.
///
/// Reads the [`ToolTree`] from the root ancestor and formats all
/// registered tools as a help string showing paths, input/output
/// types, and parameters.
///
/// # Example
///
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = StackPlugin::world();
/// let root = world.spawn((Card, children![
///     help(),
///     increment(FieldRef::new("count")),
/// ])).flush();
///
/// let help_entity = world
///     .entity(root)
///     .get::<ToolTree>()
///     .unwrap()
///     .find(&["help"])
///     .unwrap()
///     .entity;
///
/// let output = world
///     .entity_mut(help_entity)
///     .send_blocking::<(), String>(())
///     .unwrap();
///
/// assert!(output.contains("increment"));
/// ```
pub fn help() -> impl Bundle { (PathPartial::new("help"), tool(help_system)) }

/// System that reads the [`ToolTree`] from the root ancestor and formats it.
fn help_system(
	cx: In<ToolContext>,
	ancestors: Query<&ChildOf>,
	trees: Query<&ToolTree>,
) -> Result<String> {
	let root = ancestors.root_ancestor(cx.tool);
	let tree = trees.get(root).map_err(|_| {
		bevyhow!("No ToolTree found on root ancestor, cannot render help")
	})?;
	format_help(tree).xok()
}

/// Format a [`ToolTree`] as a help string, excluding the help tool itself.
fn format_help(tree: &ToolTree) -> String {
	let mut output = String::new();
	output.push_str("Available tools:\n\n");

	let nodes: Vec<&ToolNode> = tree
		.flatten_nodes()
		.into_iter()
		.filter(|node| {
			!node
				.path
				.annotated_route_path()
				.to_string()
				.ends_with("/help")
		})
		.collect();

	if nodes.is_empty() {
		output.push_str("  (none)\n");
		return output;
	}

	for node in nodes {
		format_tool_node(&mut output, node);
	}

	output
}

/// Format a single [`ToolNode`] into the output string.
fn format_tool_node(output: &mut String, node: &ToolNode) {
	let path = node.path.annotated_route_path();
	output.push_str(&format!("  {}", path));

	if let Some(method) = &node.method {
		output.push_str(&format!(" [{}]", method));
	}
	output.push('\n');

	// input/output types, skip trivial `()` types
	let input = node.meta.input().type_name();
	let output_type = node.meta.output().type_name();
	if input != "()" {
		output.push_str(&format!("    input:  {}\n", input));
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
			.get::<ToolTree>()
			.unwrap()
			.find(&["help"])
			.unwrap()
			.entity;

		let output = world
			.entity_mut(help_entity)
			.call_blocking::<(), String>(())
			.unwrap();

		output.contains("Available tools").xpect_true();
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
			.get::<ToolTree>()
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
			.spawn((Card, children![help(), add(FieldRef::new("value")),]))
			.flush();

		let help_entity = world
			.entity(root)
			.get::<ToolTree>()
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
		let root = world.spawn((Card, children![help()])).flush();

		let help_entity = world
			.entity(root)
			.get::<ToolTree>()
			.unwrap()
			.find(&["help"])
			.unwrap()
			.entity;

		let output = world
			.entity_mut(help_entity)
			.call_blocking::<(), String>(())
			.unwrap();

		output.contains("(none)").xpect_true();
	}
}
