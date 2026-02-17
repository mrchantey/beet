#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

//! Integration tests for the markdown interface.
//!
//! All tests dispatch through [`default_interface`] using [`Request`]
//! and [`Response`], verifying end-to-end behavior of the interface
//! tool chain: help, navigate, routing, and not-found fallback.

use beet_core::prelude::*;
use beet_stack::prelude::*;


fn counter() -> impl Bundle {
	let count = FieldRef::new("count").init_with(Value::I64(0));
	let count_clone = count.clone();

	(
		card("counter", move || {
			let count = count.clone();
			(children![
				Heading1::with_text("Counter"),
				(Paragraph, children![
					TextNode::new("The count is "),
					count.clone().as_text()
				]),
			],)
		}),
		children![increment(count_clone)],
	)
}


fn calculator() -> impl Bundle {
	let rhs = FieldRef::new("rhs").init_with(Value::I64(0));

	(card("calculator", || (children![],)), children![add(rhs)])
}

fn test_interface() -> (World, Entity) {
	let mut world = StackPlugin::world();
	let root = world
		.spawn((default_interface(), children![
			counter(),
			calculator(),
			card("about", || (Paragraph::with_text("About page"),)),
		]))
		.flush();
	(world, root)
}

/// Dispatch a CLI-style string through the interface and return the body.
async fn dispatch(world: &mut World, root: Entity, cli: &str) -> String {
	world
		.entity_mut(root)
		.call::<Request, Response>(Request::from_cli_str(cli).unwrap())
		.await
		.unwrap()
		.text()
		.await
		.unwrap()
}

#[beet_core::test]
async fn help_lists_all_routes() {
	let (mut world, root) = test_interface();
	let body = dispatch(&mut world, root, "--help").await;
	body.contains("Available routes").xpect_true();
	body.contains("counter").xpect_true();
	body.contains("calculator").xpect_true();
	body.contains("about").xpect_true();
	body.contains("increment").xpect_true();
	body.contains("add").xpect_true();
}

#[beet_core::test]
async fn scoped_help_for_card() {
	let (mut world, root) = test_interface();
	let body = dispatch(&mut world, root, "counter --help").await;
	// Should include tools under counter
	body.contains("increment").xpect_true();
	// Should not include sibling routes
	body.contains("calculator").xpect_false();
	body.contains("about").xpect_false();
}

#[beet_core::test]
async fn route_to_card() {
	let (mut world, root) = test_interface();
	let body = dispatch(&mut world, root, "about").await;
	body.contains("About page").xpect_true();
}

#[beet_core::test]
async fn route_to_nested_card() {
	let (mut world, root) = test_interface();
	let body = dispatch(&mut world, root, "counter").await;
	body.contains("Counter").xpect_true();
	body.contains("The count is").xpect_true();
}

#[beet_core::test]
async fn not_found_shows_contextual_help() {
	let (mut world, root) = test_interface();
	let body = dispatch(&mut world, root, "counter nonsense").await;
	body.contains("not found").xpect_true();
	// Scoped to counter
	body.contains("increment").xpect_true();
	body.contains("about").xpect_false();
}

#[beet_core::test]
async fn not_found_at_root_shows_all_routes() {
	let (mut world, root) = test_interface();
	let body = dispatch(&mut world, root, "nonexistent").await;
	body.contains("not found").xpect_true();
	body.contains("Available routes").xpect_true();
}

#[beet_core::test]
async fn navigate_first_child() {
	let (mut world, root) = test_interface();
	let body = dispatch(&mut world, root, "--navigate=first-child").await;
	// First child card should be rendered
	// (order depends on spawn order; about/calculator/counter)
	(body.contains("Counter")
		|| body.contains("About page")
		|| body.contains("add"))
	.xpect_true();
}

#[beet_core::test]
async fn navigate_parent_from_card() {
	let mut world = StackPlugin::world();
	let root = world
		.spawn((default_interface(), children![
			card("", || (Heading1::with_text("Root"),)),
			card("child", || (Paragraph::with_text("Child page"),)),
		]))
		.flush();

	let body = dispatch(&mut world, root, "child --navigate=parent").await;
	body.contains("Root").xpect_true();
}

#[beet_core::test]
async fn navigate_next_sibling() {
	let mut world = StackPlugin::world();
	let root = world
		.spawn((default_interface(), children![
			card("alpha", || (Paragraph::with_text("Alpha"),)),
			card("beta", || (Paragraph::with_text("Beta"),)),
		]))
		.flush();

	let body =
		dispatch(&mut world, root, "alpha --navigate=next-sibling").await;
	body.contains("Beta").xpect_true();
}
