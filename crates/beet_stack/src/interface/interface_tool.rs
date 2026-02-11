//! An interface routes requests to cards and tools.
//!
//! This module provides [`markdown_interface`], an async interface that
//! handles request routing, card navigation, tool invocation, and help
//! rendering. It delegates to shared functions in [`render_markdown`]
//! and [`help`] rather than duplicating their logic.
//!
//! ## Routing Behavior
//!
//! - **Empty path**: renders the `card("")` child if present, otherwise
//!   falls back to rendering the root entity directly.
//! - **`--help`**: scoped to the requested path prefix, ie
//!   `counter --help` only shows routes under `/counter`.
//! - **Not found**: shows help scoped to the nearest ancestor card,
//!   ie `counter nonsense` shows help for `/counter`.

use crate::prelude::*;
use beet_core::prelude::*;

/// An interface for interacting with a card-based application.
///
/// Interfaces provide a way to navigate between cards and call tools
/// within the current card context. The [`Interface`] component tracks
/// the currently active card, enabling REPL-like navigation:
///
/// ```text
/// > my_app
/// // prints help for root: subcommands: foo
/// > foo
/// // prints foo in markdown, and sets current card to foo
/// > --help
/// // prints help for foo, not the root: subcommands: bar
/// > bar
/// // goes to route foo/bar, if bar is a tool dont update current card.
/// ```
///
/// Help is handled by the interface itself, not added to each route.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Interface {}

impl Interface {}

/// Create an interface from a handler, inserting an [`Interface`]
/// pointing to itself as the current card.
pub fn interface() -> impl Bundle {
	(
		Interface::default(),
		ExchangeToolMarker,
		RouteHidden,
		direct_tool(async |request: AsyncToolContext<Request>| -> Response {
			match fallback::<Request, Response>(request).await {
				Ok(Pass(res)) => res,
				Ok(Fail(_req)) => Response::not_found(),
				// if the returned error is a HttpError its status code will be used.
				Err(err) => HttpError::from_opaque(err).into_response(),
			}
		}),
	)
}

/// Creates a standard markdown interface with help, routing, and
/// fallback handlers as a child fallback chain.
///
/// The handler chain runs in order:
/// 1. **Help** — if `--help` is present, render help scoped to the
///    request path prefix.
/// 2. **Router** — look up the path in the [`RouteTree`]. Cards are
///    rendered as markdown, tools are called directly. An empty path
///    resolves to `card("")` when present.
/// 3. **Contextual Not Found** — show help for the nearest ancestor
///    card of the unmatched path.
pub fn markdown_interface() -> impl Bundle {
	(
		interface(),
		OnSpawn::insert(children![
			(
				Name::new("Help Tool"),
				RouteHidden,
				direct_tool(help_handler)
			),
			(Name::new("Router"), RouteHidden, direct_tool(route_handler)),
			(
				Name::new("Contextual Not Found"),
				RouteHidden,
				direct_tool(nearest_ancestor_help_handler)
			)
		]),
	)
}

/// Walks up [`ChildOf`] relations to find the root ancestor entity.
fn walk_to_root(world: &World, entity: Entity) -> Entity {
	let mut current = entity;
	while let Some(child_of) = world.entity(current).get::<ChildOf>() {
		current = child_of.parent();
	}
	current
}

/// Gets the [`RouteTree`] from the root ancestor of the given entity.
fn root_route_tree(world: &World, entity: Entity) -> Result<&RouteTree> {
	let root = walk_to_root(world, entity);
	world
		.entity(root)
		.get::<RouteTree>()
		.ok_or_else(|| bevyhow!("No RouteTree found on root ancestor"))
}

async fn help_handler(
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

async fn route_handler(
	cx: AsyncToolContext<Request>,
) -> Result<Outcome<Response, Request>> {
	let path = cx.input.path().clone();
	let is_empty_path = path.is_empty();
	let tool_entity = cx.tool.id();
	let world = cx.tool.world();

	// Look up the path in the root ancestor's route tree.
	// Use `.ok()` so a missing RouteTree returns None instead of error,
	// allowing the empty-path fallback below to render the root entity.
	let node = world
		.with_then(move |world: &mut World| -> Option<RouteNode> {
			let tree = root_route_tree(world, tool_entity).ok()?;
			tree.find(&path).cloned()
		})
		.await;

	match node {
		Some(RouteNode::Card(card_node)) => {
			let card_entity = card_node.entity;
			let markdown = world
				.with_then(move |world: &mut World| {
					render_markdown_for(card_entity, world)
				})
				.await;
			Pass(Response::ok_body(markdown, "text/plain"))
		}
		Some(RouteNode::Tool(tool_node)) => Pass(
			world
				.entity(tool_node.entity)
				.call::<Request, Response>(cx.input)
				.await?,
		),
		// Empty path with no card("") in tree: render the root entity
		None if is_empty_path => {
			let markdown = world
				.with_then(move |world: &mut World| {
					let root = walk_to_root(world, tool_entity);
					render_markdown_for(root, world)
				})
				.await;
			Pass(Response::ok_body(markdown, "text/plain"))
		}
		None => Fail(cx.input),
	}
	.xok()
}

async fn nearest_ancestor_help_handler(
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

	Outcome::Pass(Response::ok_body(help_text, "text/plain")).xok()
}

/// Walks the path segments from longest to shortest prefix, returning
/// help for the first ancestor that matches a card. Falls back to
/// root help if nothing matches.
fn nearest_ancestor_help(tree: &RouteTree, segments: &Vec<String>) -> String {
	// Try progressively shorter prefixes
	for length in (1..segments.len()).rev() {
		let prefix = &segments[..length];
		if let Some(RouteNode::Card(_)) = tree.find(prefix) {
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

	// Nothing matched at all - show root help with a not-found preamble
	let mut output = String::new();
	output.push_str(&format!("Route /{} not found.\n\n", segments.join("/"),));
	output.push_str(&format_route_help(tree));
	output
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn my_interface() -> impl Bundle {
		(
			Interface::default(),
			direct_tool(
				|req: In<ToolContext<Request>>,
				 trees: Query<&RouteTree>,
				 interfaces: Query<&Interface>|
				 -> Result<RouteTree> {
					let _interface = interfaces.get(req.tool)?;
					let tree = trees.get(req.tool)?;
					Ok(tree.clone())
				},
			),
			children![(
				PathPartial::new("add"),
				tool(|(a, b): (u32, u32)| a + b)
			)],
		)
	}


	#[test]
	fn works() {
		let tree = RouterPlugin::world()
			.spawn(my_interface())
			.call_blocking::<_, RouteTree>(Request::get("foo"))
			.unwrap();
		tree.find_tool(&["add"]).xpect_some();
		tree.find_tool(&["add"])
			.unwrap()
			.path
			.annotated_route_path()
			.to_string()
			.xpect_eq("/add");
	}

	#[beet_core::test]
	async fn markdown_interface_renders_help() {
		let mut world = StackPlugin::world();

		let root = world
			.spawn((Card, markdown_interface(), children![
				increment(FieldRef::new("count")),
				card("about"),
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
	async fn markdown_interface_renders_card() {
		StackPlugin::world()
			.spawn((Card, markdown_interface(), children![(
				card("about"),
				Paragraph,
				TextContent::new("About page"),
			)]))
			.call::<Request, Response>(Request::get("about"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.contains("About page")
			.xpect_true();
	}

	#[beet_core::test]
	async fn markdown_interface_renders_current_on_empty_path() {
		StackPlugin::world()
			.spawn((
				Card,
				markdown_interface(),
				Paragraph,
				TextContent::new("Root content"),
			))
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("Root content");
	}

	#[beet_core::test]
	async fn markdown_interface_not_found_shows_ancestor_help() {
		StackPlugin::world()
			.spawn((Card, markdown_interface(), children![increment(
				FieldRef::new("count")
			),]))
			.call::<Request, Response>(
				Request::from_cli_str("nonexistent").unwrap(),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("not found")
			.xpect_contains("Available routes");
	}

	#[beet_core::test]
	async fn markdown_interface_calls_exchange_tool() {
		StackPlugin::world()
			.spawn((Card, markdown_interface(), children![(
				PathPartial::new("add"),
				exchange_tool(|input: (i32, i32)| -> i32 { input.0 + input.1 }),
			)]))
			.call::<Request, Response>(
				Request::with_json("/add", &(10i32, 20i32)).unwrap(),
			)
			.await
			.unwrap()
			.body
			.into_json::<i32>()
			.await
			.unwrap()
			.xpect_eq(30);
	}

	#[beet_core::test]
	async fn renders_root_card_child() {
		let body = StackPlugin::world()
			.spawn((Card, markdown_interface(), children![
				(card(""), Title::with_text("My Server"), children![
					Paragraph::with_text("welcome!")
				]),
				card("about"),
			]))
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("My Server").xpect_true();
		body.contains("welcome!").xpect_true();
	}

	#[beet_core::test]
	async fn help_scoped_to_prefix() {
		let body = StackPlugin::world()
			.spawn((Card, markdown_interface(), children![
				(card("counter"), children![increment(FieldRef::new(
					"count"
				)),]),
				card("about"),
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
	async fn not_found_shows_scoped_ancestor_help() {
		let body = StackPlugin::world()
			.spawn((Card, markdown_interface(), children![
				(card("counter"), children![increment(FieldRef::new(
					"count"
				)),]),
				card("about"),
			]))
			.call::<Request, Response>(
				Request::from_cli_str("counter nonsense").unwrap(),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("not found").xpect_true();
		// Should show routes under counter, not the full tree
		body.contains("increment").xpect_true();
		body.contains("about").xpect_false();
	}
}
