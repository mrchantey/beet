#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

use beet_core::prelude::*;
use beet_stack::prelude::*;


fn counter() -> impl Bundle {
	let value = FieldRef::new("count").init_with(Value::I64(0));

	(Card, PathPartial::new("counter"), children![
		render_markdown(),
		increment(value)
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
fn tool_tree_built_on_spawn() {
	let (world, root) = test_stack();
	let tree = world.entity(root).get::<ToolTree>().unwrap();
	// help, render-markdown, counter/increment, counter/render-markdown,
	// calculator/add, calculator/render-markdown
	tree.flatten().len().xpect_eq(6);
}

#[test]
fn find_tool_by_path() {
	let (world, root) = test_stack();
	let tree = world.entity(root).get::<ToolTree>().unwrap();
	tree.find(&["counter", "increment"]).xpect_some();
	tree.find(&["calculator", "add"]).xpect_some();
	tree.find(&["help"]).xpect_some();
	tree.find(&["nonexistent"]).xpect_none();
}

#[test]
fn help_lists_all_tools() {
	let (mut world, root) = test_stack();
	let help_entity = world
		.entity(root)
		.get::<ToolTree>()
		.unwrap()
		.find(&["help"])
		.unwrap()
		.entity;

	let output = world
		.entity_mut(help_entity)
		.send_blocking::<(), String>(())
		.unwrap();

	output.contains("Available tools").xpect_true();
	output.contains("counter/increment").xpect_true();
	output.contains("calculator/add").xpect_true();
	// help itself should not appear in its own output
	output.contains("/help").xpect_false();
}

#[test]
fn dispatch_increment_via_cli_path() {
	let (mut world, root) = test_stack();
	let cli = CliArgs::parse("counter increment");

	let tree = world.entity(root).get::<ToolTree>().unwrap().clone();

	let node = tree.find(&cli.path).unwrap();
	let tool_entity = node.entity;

	world
		.entity_mut(tool_entity)
		.send_blocking::<(), i64>(())
		.unwrap()
		.xpect_eq(1);

	world
		.entity_mut(tool_entity)
		.send_blocking::<(), i64>(())
		.unwrap()
		.xpect_eq(2);
}

#[test]
fn dispatch_add_via_cli_path() {
	let (mut world, root) = test_stack();
	let cli = CliArgs::parse("calculator add");

	let tree = world.entity(root).get::<ToolTree>().unwrap().clone();

	let node = tree.find(&cli.path).unwrap();
	let tool_entity = node.entity;

	world
		.entity_mut(tool_entity)
		.send_blocking::<i64, i64>(5)
		.unwrap()
		.xpect_eq(5);

	world
		.entity_mut(tool_entity)
		.send_blocking::<i64, i64>(3)
		.unwrap()
		.xpect_eq(8);
}

#[test]
fn dispatch_help_via_cli_path() {
	let (mut world, root) = test_stack();
	let cli = CliArgs::parse("help");

	let tree = world.entity(root).get::<ToolTree>().unwrap().clone();

	let node = tree.find(&cli.path).unwrap();
	let output = world
		.entity_mut(node.entity)
		.send_blocking::<(), String>(())
		.unwrap();

	output.contains("Available tools").xpect_true();
	output.contains("increment").xpect_true();
}

#[test]
fn cli_path_not_found() {
	let (world, root) = test_stack();
	let cli = CliArgs::parse("nonexistent command");

	let tree = world.entity(root).get::<ToolTree>().unwrap();
	tree.find(&cli.path).xpect_none();
}
