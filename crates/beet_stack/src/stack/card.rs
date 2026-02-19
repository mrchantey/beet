use crate::prelude::*;
use beet_core::prelude::*;

/// An entity spawned by a [`card`] tool call.
///
/// Points back to the card tool entity that produced it, establishing
/// the [`Cards`] relationship on the card tool.
#[derive(Debug, Clone, Deref, Component)]
#[relationship(relationship_target = Cards)]
pub struct CardOf(pub Entity);

impl CardOf {
	/// Creates a new [`CardOf`] relationship pointing to the given card tool entity.
	pub fn new(value: Entity) -> Self { Self(value) }
}

/// All card content entities currently spawned by this card tool.
///
/// Automatically maintained by the [`CardOf`] relationship. When the card
/// tool entity is despawned, all tracked card entities are also despawned.
#[derive(Debug, Clone, Deref, Component)]
#[relationship_target(relationship = CardOf, linked_spawn)]
pub struct Cards(Vec<Entity>);

/// A single content container, similar to pages in a website or cards
/// in HyperCard. Each card is a route, with the exact rendering behavior
/// determined by the render tool on the server or interface.
///
/// Use the [`card`] function to create a routable card with content:
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = StackPlugin::world();
/// let root = world.spawn((
///     default_router(),
///     children![
///         card("about", || Paragraph::with_text("About page")),
///     ],
/// )).flush();
///
/// let tree = world.entity(root).get::<RouteTree>().unwrap();
/// tree.find(&["about"]).xpect_some();
/// ```
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Card;

/// Marker component for render tools on servers or interfaces.
///
/// A render tool accepts a [`RenderRequest`] and returns a [`Response`].
/// Different servers provide different render tools:
/// - CLI/REPL: renders content to markdown, despawns the entity
/// - TUI: manages stateful card display
///
/// Found by [`find_render_tool`] which traverses to the root ancestor
/// and searches descendants for this marker.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct RenderToolMarker;

/// Request passed to a render tool to render a card's content.
///
/// The render tool decides whether and when to call [`spawn_tool`](RenderRequest::spawn_tool)
/// to materialise the card's content as an entity. This gives render tools
/// full control over spawn/despawn lifecycle — eg stateless renderers
/// despawn immediately; stateful ones retain the entity as [`CurrentCard`].
#[derive(Debug)]
pub struct RenderRequest {
	/// The card tool entity that issued this request.
	pub card_tool: Entity,
	/// A tool that, when called with `()`, spawns the card bundle as an entity
	/// and attaches [`CardOf(card_tool)`](CardOf) to it.
	/// The render tool chooses when (or whether) to call this.
	pub spawn_tool: Tool<(), Entity>,
	/// The original request.
	pub request: Request,
}

/// Creates a routable card tool from a path and content handler.
///
/// The handler is a function that returns an [`impl Bundle`](Bundle).
/// When a request arrives the card tool:
/// 1. Locates the nearest [`RenderToolMarker`] via [`find_render_tool`].
/// 2. Builds a `spawn_tool` that, on demand, spawns the bundle with
///    [`CardOf`] pointing back to the card entity.
/// 3. Forwards a [`RenderRequest`] to the render tool, which decides
///    when to call `spawn_tool` and how to manage the resulting entity.
///
/// # Example
///
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = StackPlugin::world();
/// let root = world.spawn((
///     default_router(),
///     children![
///         card("about", || Paragraph::with_text("About page")),
///         card("home", || children![
///             Heading1::with_text("Welcome"),
///             Paragraph::with_text("Hello!"),
///         ]),
///     ],
/// )).flush();
///
/// let tree = world.entity(root).get::<RouteTree>().unwrap();
/// tree.find(&["about"]).xpect_some();
/// tree.find(&["home"]).xpect_some();
/// ```
pub fn card<F, B>(path: &str, func: F) -> impl Bundle
where
	F: 'static + Send + Sync + Clone + Fn() -> B,
	B: 'static + Send + Sync + Bundle,
{
	let handler = {
		let func = func.clone();
		Tool::new(
			TypeMeta::of::<F>(),
			move |ToolCall {
			          mut commands,
			          tool: card_tool,
			          input: request,
			          out_handler,
			      }| {
				let func = func.clone();
				commands.run(async move |world: AsyncWorld| -> Result {
					// Build spawn_tool inline, capturing card_tool and func.
					// The render tool calls this when it wants to materialise content.
					let spawn_tool = Tool::new(TypeMeta::of::<F>(), {
						let func = func.clone();
						move |ToolCall {
						          mut commands,
						          tool: _,
						          input: (),
						          out_handler,
						      }| {
							let func = func.clone();
							commands.commands.queue(
								move |world: &mut World| -> Result {
									let entity = world
										.spawn((
											Card,
											CardOf::new(card_tool),
											func(),
										))
										.id();
									out_handler.call_world(world, entity)
								},
							);
							Ok(())
						}
					});

					// Locate the nearest render tool in the hierarchy
					let render_tool = world
						.with_then(move |world: &mut World| {
							find_render_tool(world, card_tool)
						})
						.await?;

					// Delegate to the render tool with spawn capability
					let response: Response = world
						.entity(render_tool)
						.call::<RenderRequest, Response>(RenderRequest {
							card_tool,
							spawn_tool,
							request,
						})
						.await?;

					out_handler.call_async(world, response).await
				});
				Ok(())
			},
		)
	};

	(
		PathPartial::new(path),
		Card,
		handler,
	)
}

/// Creates a routable card that loads its content from a file.
///
/// On each render, the file is read from disk and its text content
/// is displayed. Internally calls [`card`] with a handler that
/// reads the file.
///
/// # Example
///
/// ```no_run
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let bundle = file_card("readme", "docs/readme.md");
/// ```
pub fn file_card(path: &str, file_path: impl Into<WsPathBuf>) -> impl Bundle {
	let ws_path: WsPathBuf = file_path.into();
	card(path, file_card_content_tool(ws_path))
}

/// Creates a content handler that reads a file from disk and
/// returns its text as a [`TextNode`].
fn file_card_content_tool(
	ws_path: WsPathBuf,
) -> impl 'static + Send + Sync + Clone + Fn() -> TextNode {
	move || {
		let abs_path = ws_path.clone().into_abs();
		match fs_ext::read_to_string(&abs_path) {
			Ok(text) => TextNode::new(text),
			Err(err) => {
				cross_log_error!("Failed to load file: {err}");
				TextNode::new(format!(
					"Error loading {}: {err}",
					abs_path.display()
				))
			}
		}
	}
}

/// Finds the nearest render tool by traversing to the root ancestor
/// and searching descendants for a [`RenderToolMarker`].
///
/// # Errors
///
/// Returns an error if no render tool is found in the hierarchy.
/// Ensure a render tool is added to the server, ie
/// [`markdown_render_tool`] for CLI/REPL or [`tui_render_tool`]
/// for TUI.
pub fn find_render_tool(world: &mut World, entity: Entity) -> Result<Entity> {
	world
		.run_system_once_with(
			|In(entity): In<Entity>,
			 ancestors: Query<&ChildOf>,
			 children: Query<&Children>,
			 markers: Query<Entity, With<RenderToolMarker>>| {
				let root = ancestors.root_ancestor(entity);
				children
					.iter_descendants_inclusive(root)
					.find(|&desc| markers.contains(desc))
			},
			entity,
		)
		.ok()
		.flatten()
		.ok_or_else(|| {
			bevyhow!(
				"No render tool found. Add a render tool like \
				 `markdown_render_tool()` to the server."
			)
		})
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn card_renders_via_render_tool() {
		StackPlugin::world()
			.spawn((default_router(), children![
				markdown_render_tool(),
				card("about", || Paragraph::with_text("About page")),
			]))
			.call::<Request, Response>(Request::get("about"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("About page");
	}

	#[beet_core::test]
	async fn card_with_children() {
		StackPlugin::world()
			.spawn((default_router(), children![
				markdown_render_tool(),
				card("home", || {
					children![
						Heading1::with_text("Welcome"),
						Paragraph::with_text("Hello!"),
					]
				}),
			]))
			.call::<Request, Response>(Request::get("home"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("Welcome")
			.xpect_contains("Hello!");
	}

	#[test]
	fn card_appears_in_route_tree() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((default_router(), children![
				markdown_render_tool(),
				card("about", || Paragraph::with_text("About page")),
			]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.find(&["about"]).xpect_some();
	}

	#[test]
	fn find_render_tool_traverses_hierarchy() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((default_router(), children![
				markdown_render_tool(),
				card("test", || Paragraph::with_text("test")),
			]))
			.flush();

		let result = find_render_tool(&mut world, root);
		result.xpect_ok();
	}

	#[test]
	fn find_render_tool_errors_without_render_tool() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		find_render_tool(&mut world, entity).xpect_err();
	}

	#[beet_core::test]
	async fn spawned_card_has_card_of_relationship() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((default_router(), children![
				markdown_render_tool(),
				card("about", || Paragraph::with_text("About page")),
			]))
			.flush();

		// find the card tool entity
		let card_entity = world
			.run_system_once(
				|cards: Query<Entity, (With<Card>, With<PathPartial>)>| {
					cards.iter().next()
				},
			)
			.unwrap();

		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("about"))
			.await
			.unwrap();

		// after rendering, the card tool should have no Cards remaining
		// (markdown render tool despawns the entity after rendering)
		let cards = world.entity(card_entity.unwrap()).get::<Cards>();
		// either no Cards component or empty — both are valid after despawn
		let count = cards.map(|c| c.len()).unwrap_or(0);
		count.xpect_eq(0);
	}
}
