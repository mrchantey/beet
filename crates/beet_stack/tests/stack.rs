#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

use beet_core::prelude::*;
use beet_stack::prelude::*;


fn counter() -> impl Bundle {
	let count = FieldRef::new("count").init_with(Value::I64(0));

	(Card, PathPartial::new("counter"), children![
		(Title::with_text("Counter"), children![
			Paragraph::with_text("The count is "),
			count.clone().as_text()
		]),
		render_markdown(),
		increment(count)
	])
}


fn calculator() -> impl Bundle {
	let rhs = FieldRef::new("rhs").init_with(Value::I64(0));

	(Card, PathPartial::new("calculator"), children![
		render_markdown(),
		add(rhs)
	])
}

fn test_stack() -> (World, Entity) {
	let mut world = StackPlugin::world();
	let root = world
		.spawn((Card, children![
			help(),
			render_markdown(),
			counter(),
			calculator()
		]))
		.flush();
	(world, root)
}

#[test]
fn route_tree_built_on_spawn() {
	let (world, root) = test_stack();
	let tree = world.entity(root).get::<RouteTree>().unwrap();
	// help, render-markdown, counter/increment, counter/render-markdown,
	// calculator/add, calculator/render-markdown,
	// plus card routes: counter, calculator
	tree.flatten_tool_nodes().len().xpect_eq(6);
}

#[test]
fn find_tool_by_path() {
	let (world, root) = test_stack();
	let tree = world.entity(root).get::<RouteTree>().unwrap();
	tree.find_tool(&["counter", "increment"]).xpect_some();
	tree.find_tool(&["calculator", "add"]).xpect_some();
	tree.find_tool(&["help"]).xpect_some();
	tree.find(&["nonexistent"]).xpect_none();
}

#[test]
fn find_card_by_path() {
	let (world, root) = test_stack();
	let tree = world.entity(root).get::<RouteTree>().unwrap();
	tree.find_card(&["counter"]).xpect_some();
	tree.find_card(&["calculator"]).xpect_some();
	// tools are not cards
	tree.find_card(&["help"]).xpect_none();
}

#[test]
fn help_lists_all_tools() {
	let (mut world, root) = test_stack();
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
	output.contains("counter/increment").xpect_true();
	output.contains("calculator/add").xpect_true();
	// help itself should not appear in its own output
	output.contains("/help").xpect_false();
}

#[test]
fn dispatch_increment_via_cli_path() {
	let (mut world, root) = test_stack();
	let cli = CliArgs::parse("counter increment");

	let tree = world.entity(root).get::<RouteTree>().unwrap().clone();

	let node = tree.find_tool(&cli.path).unwrap();
	let tool_entity = node.entity;

	world
		.entity_mut(tool_entity)
		.call_blocking::<(), i64>(())
		.unwrap()
		.xpect_eq(1);

	world
		.entity_mut(tool_entity)
		.call_blocking::<(), i64>(())
		.unwrap()
		.xpect_eq(2);
}

#[test]
fn dispatch_add_via_cli_path() {
	let (mut world, root) = test_stack();
	let cli = CliArgs::parse("calculator add");

	let tree = world.entity(root).get::<RouteTree>().unwrap().clone();

	let node = tree.find_tool(&cli.path).unwrap();
	let tool_entity = node.entity;

	world
		.entity_mut(tool_entity)
		.call_blocking::<i64, i64>(5)
		.unwrap()
		.xpect_eq(5);

	world
		.entity_mut(tool_entity)
		.call_blocking::<i64, i64>(3)
		.unwrap()
		.xpect_eq(8);
}

#[test]
fn dispatch_help_via_cli_path() {
	let (mut world, root) = test_stack();
	let cli = CliArgs::parse("help");

	let tree = world.entity(root).get::<RouteTree>().unwrap().clone();

	let node = tree.find_tool(&cli.path).unwrap();
	let output = world
		.entity_mut(node.entity)
		.call_blocking::<(), String>(())
		.unwrap();

	output.contains("Available routes").xpect_true();
	output.contains("increment").xpect_true();
}

#[test]
fn cli_path_not_found() {
	let (world, root) = test_stack();
	let cli = CliArgs::parse("nonexistent command");

	let tree = world.entity(root).get::<RouteTree>().unwrap();
	tree.find(&cli.path).xpect_none();
}

#[test]
fn cards_are_routes() {
	let (world, root) = test_stack();
	let tree = world.entity(root).get::<RouteTree>().unwrap();

	// cards should appear as routes: root + counter + calculator
	let card_nodes = tree.flatten_card_nodes();
	card_nodes.len().xpect_eq(3);

	// and be findable
	let counter_card = tree.find_card(&["counter"]).unwrap();
	world
		.entity(counter_card.entity)
		.contains::<Card>()
		.xpect_true();
}

#[test]
fn route_node_entity_is_queryable() {
	let (world, root) = test_stack();
	let tree = world.entity(root).get::<RouteTree>().unwrap();

	let tool_node = tree.find(&["counter", "increment"]).unwrap();
	world
		.entity(tool_node.entity())
		.contains::<ToolMeta>()
		.xpect_true();

	let card_node = tree.find(&["counter"]).unwrap();
	world
		.entity(card_node.entity())
		.contains::<Card>()
		.xpect_true();
}
