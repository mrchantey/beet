//! An interface routes requests to cards and tools.
//!
//! This module provides [`markdown_interface`], an async interface that
//! handles request routing, card navigation, tool invocation, and help
//! rendering. It delegates to shared functions in [`render_markdown`]
//! and [`help`] rather than duplicating their logic.

use crate::prelude::*;
use beet_core::exports::async_channel;
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
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Interface {
	/// The card currently being accessed by the interface,
	/// defaulting to the root.
	current_card: Entity,
}

impl Interface {
	/// Create a new [`Interface`] pointing to the given card entity.
	pub fn new(card: Entity) -> Self { Self { current_card: card } }

	/// Returns the current card entity.
	pub fn current_card(&self) -> Entity { self.current_card }

	/// Sets the current card entity.
	pub fn set_current_card(&mut self, card: Entity) {
		self.current_card = card;
	}

	/// Create a new [`Interface`] pointing to the entity it was inserted on.
	pub fn new_this() -> impl Bundle {
		OnSpawn::new(|entity| {
			let id = entity.id();
			entity.insert(Interface::new(id));
		})
	}
}


/// Create an interface from a handler, inserting an [`Interface`]
/// pointing to itself as the current card.
pub fn interface<H, M>(handler: H) -> impl Bundle
where
	H: IntoToolHandler<M, In = Request, Out = Response>,
{
	(Interface::new_this(), direct_tool(handler))
}


/// Creates a standard markdown interface that handles routing,
/// help, and card navigation asynchronously.
///
/// This interface handles requests as follows:
/// 1. If the request has a `help` param, renders help for the current card
/// 2. If the path matches a card, renders its markdown and updates
///    [`Interface::current_card`]
/// 3. If the path matches an exchange tool, calls it and returns the response
/// 4. If no path matches, returns help for the nearest matching ancestor
///
/// Paths are resolved relative to the current card.
pub fn markdown_interface() -> impl Bundle {
	(
		Interface::new_this(),
		ToolMeta::of::<Request, Response>(),
		ExchangeToolMarker,
		OnSpawn::observe(
			|mut ev: On<ToolIn<Request, Response>>,
			 mut commands: AsyncCommands|
			 -> Result {
				let ev = ev.event_mut();
				let tool = ev.tool();
				let request = ev.take_input()?;
				let out_handler = ev.take_out_handler()?;

				commands.run(async move |mut world: AsyncWorld| -> Result {
					let response = route_request(&world, tool, request).await?;
					out_handler.call_async(&mut world, tool, response)?;
					Ok(())
				});
				Ok(())
			},
		),
	)
}

/// The result of routing a request against the interface.
enum InterfaceAction {
	/// Render markdown for a card and update current_card.
	NavigateCard { card_entity: Entity },
	/// Forward the request to an exchange tool.
	CallTool {
		tool_entity: Entity,
		request: Request,
	},
	/// Render the current card (empty path).
	RenderCurrent { card_entity: Entity },
	/// Show help text.
	Help(String),
}

/// Routes a request through the interface, returning a [`Response`].
async fn route_request(
	world: &AsyncWorld,
	interface_entity: Entity,
	request: Request,
) -> Result<Response> {
	// Determine what action to take based on the request
	let action: InterfaceAction = world
		.entity(interface_entity)
		.with_then(move |entity| -> Result<InterfaceAction> {
			resolve_action(&entity, request)
		})
		.await?;

	match action {
		InterfaceAction::NavigateCard { card_entity } => {
			// Update current_card, then render markdown
			let markdown = world
				.with_then(move |world: &mut World| {
					world
						.entity_mut(interface_entity)
						.get_mut::<Interface>()
						.unwrap()
						.set_current_card(card_entity);
					render_markdown_for(card_entity, world)
				})
				.await;
			Response::ok().with_body(markdown).xok()
		}
		InterfaceAction::CallTool {
			tool_entity,
			request,
		} => {
			// Trigger the tool and await the response via a channel.
			// The outer poll_and_update (from call_blocking) drives
			// world updates, so we only need to trigger + await here.
			let (send, recv) = async_channel::bounded::<Response>(1);
			world
				.with_then(move |world: &mut World| -> Result {
					let handler = ToolOutHandler::channel(send);
					let mut entity = world.entity_mut(tool_entity);
					entity.trigger(|entity| {
						ToolIn::new(entity, request, handler)
					});
					entity.flush();
					Ok(())
				})
				.await?;

			let response = recv
				.recv()
				.await
				.map_err(|_| bevyhow!("Tool response channel closed"))?;
			response.xok()
		}
		InterfaceAction::RenderCurrent { card_entity } => {
			let markdown = world
				.with_then(move |world: &mut World| {
					render_markdown_for(card_entity, world)
				})
				.await;
			Response::ok().with_body(markdown).xok()
		}
		InterfaceAction::Help(text) => Response::ok().with_body(text).xok(),
	}
}

/// Determines the [`InterfaceAction`] for a request by inspecting the
/// route tree and interface state. Runs synchronously with entity access.
fn resolve_action(
	entity: &EntityWorldMut,
	request: Request,
) -> Result<InterfaceAction> {
	let interface = entity
		.get::<Interface>()
		.ok_or_else(|| bevyhow!("No Interface component on entity"))?;
	let current_card = interface.current_card();

	// Find root ancestor by walking up ChildOf relationships
	let world = entity.world();
	let root = {
		let mut current = entity.id();
		while let Some(child_of) = world.entity(current).get::<ChildOf>() {
			current = child_of.parent();
		}
		current
	};

	let tree = world
		.entity(root)
		.get::<RouteTree>()
		.ok_or_else(|| bevyhow!("No RouteTree found on root ancestor"))?;

	// Handle help requests
	if request.has_param("help") {
		let help_text = format_route_help(tree);
		return InterfaceAction::Help(help_text).xok();
	}

	let path = request.route_path();
	let segments = path.segments();

	// Empty path -> render current card
	if segments.is_empty() {
		return InterfaceAction::RenderCurrent {
			card_entity: current_card,
		}
		.xok();
	}

	// Try to find a matching route
	if let Some(route_node) = tree.find(&segments) {
		return match route_node {
			RouteNode::Card(card_node) => InterfaceAction::NavigateCard {
				card_entity: card_node.entity,
			}
			.xok(),
			RouteNode::Tool(tool_node) => {
				if tool_node.is_exchange {
					InterfaceAction::CallTool {
						tool_entity: tool_node.entity,
						request,
					}
					.xok()
				} else {
					bevybail!(
						"Tool at /{} is not an exchange tool and cannot be called via the interface",
						segments.join("/")
					)
				}
			}
		};
	}

	// Route not found - find nearest matching ancestor and show its help
	let help_text = nearest_ancestor_help(tree, &segments);
	InterfaceAction::Help(help_text).xok()
}

/// Walks the path segments from longest to shortest prefix, returning
/// help for the first ancestor that matches a card. Falls back to
/// root help if nothing matches.
fn nearest_ancestor_help(tree: &RouteTree, segments: &[&str]) -> String {
	// Try progressively shorter prefixes
	for length in (1..segments.len()).rev() {
		let prefix = &segments[..length];
		if let Some(RouteNode::Card(_)) = tree.find(prefix) {
			// Found a card ancestor - build a subtree-scoped help message
			let prefix_str = prefix.join("/");
			let mut output = String::new();
			output.push_str(&format!(
				"Route /{} not found. Showing help for /{}:\n\n",
				segments.join("/"),
				prefix_str,
			));
			// Append the full tree help (filtered routes would be better
			// but the current RouteTree API doesn't expose subtrees)
			output.push_str(&format_route_help(tree));
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
			Interface::new_this(),
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

	#[test]
	fn interface_tracks_current_card() {
		let mut world = StackPlugin::world();
		let root = world.spawn((Interface::new_this(), Card)).flush();

		let interface = world.entity(root).get::<Interface>().unwrap();
		interface.current_card().xpect_eq(root);
	}

	#[test]
	fn interface_card_navigation() {
		let mut world = StackPlugin::world();
		let root = world.spawn((Interface::new_this(), Card)).flush();

		let child_card = world.spawn((ChildOf(root), card("about"))).flush();

		let mut binding = world.entity_mut(root);
		let mut interface = binding.get_mut::<Interface>().unwrap();
		interface.set_current_card(child_card);
		drop(interface);
		drop(binding);

		let interface = world.entity(root).get::<Interface>().unwrap();
		interface.current_card().xpect_eq(child_card);
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
}
