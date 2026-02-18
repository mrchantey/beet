use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemState;
use std::sync::Arc;

/// A single content container, similar to pages in a website or cards
/// in HyperCard. Each card is a route, with the exact rendering behavior
/// determined by the render tool on the server or interface.
///
/// Cards are tools that delegate rendering to a [`RenderToolMarker`]
/// entity found by traversing to the root ancestor. The `Card` component
/// serves both as a boundary marker for
/// [`CardWalker`](crate::renderers::CardWalker) and [`DocumentQuery`]
/// traversal, and as a marker on card tool entities to distinguish
/// them from regular tools in help display and route tree queries.
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

/// Marker for the internal child tool entity that spawns card content.
///
/// The render tool calls this child entity (via [`RenderRequest::handler`])
/// to spawn the card's content tree. The child tool takes `()` and
/// returns an [`Entity`] representing the spawned card content root.
///
/// Also stores a [`CardContentFn`] for synchronous spawning during
/// route discovery.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct CardContentHandler;

/// Type-erased synchronous card content spawner.
///
/// Used during [`RouteTree`] construction to discover nested tools
/// and cards inside card content. The route tree observer calls this
/// to temporarily spawn content, collect routes, then despawn.
///
/// This duplicates the async tool handler's logic but provides
/// synchronous `&mut World` access needed during route discovery.
#[derive(Component, Clone)]
pub struct CardContentFn(Arc<dyn Fn(&mut World) -> Entity + Send + Sync>);

impl CardContentFn {
	/// Create a new content function.
	pub fn new(
		func: impl Fn(&mut World) -> Entity + 'static + Send + Sync,
	) -> Self {
		Self(Arc::new(func))
	}

	/// Spawn the card content into the world, returning the root entity.
	pub fn spawn(&self, world: &mut World) -> Entity { (self.0)(world) }
}

impl std::fmt::Debug for CardContentFn {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CardContentFn").finish_non_exhaustive()
	}
}

/// Marker component for render tools on servers or interfaces.
///
/// A render tool accepts a [`RenderRequest`] and returns a [`Response`].
/// Different servers provide different render tools:
/// - CLI/REPL: spawns content, renders to markdown, despawns
/// - TUI: manages stateful card display
///
/// Found by [`find_render_tool`] which traverses to the root ancestor
/// and searches descendants for this marker.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct RenderToolMarker;

/// Request passed to a render tool to render a card's content.
///
/// The render tool calls the [`CardContentHandler`] child entity
/// referenced by [`handler`](Self::handler) to spawn the card's
/// content tree, enabling stateful renderers (like TUI) to manage
/// lifecycle differently from stateless ones (like markdown).
#[derive(Debug)]
pub struct RenderRequest {
	/// The [`CardContentHandler`] child entity. Call this entity
	/// with `call::<(), Entity>(())` to spawn the card content.
	pub handler: Entity,
	/// Cards must be called once at first to discover their
	/// nested tools. When true, the render tool should avoid
	/// unnecessary on-mount work.
	pub discover_call: bool,
	/// The original request.
	pub request: Request,
}

/// Creates a routable card tool from a path and content handler.
///
/// The handler is a function that returns an [`impl Bundle`](Bundle),
/// which will be spawned as the card's content when rendered. The
/// card registers as a [`Request`]/[`Response`] tool in the
/// [`RouteTree`], delegating rendering to the nearest
/// [`RenderToolMarker`] found via ancestor traversal.
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
	(
		PathPartial::new(path),
		Card,
		// Outer tool meta: the card presents as Request/Response
		ToolMeta::of::<F, Request, Response>(),
		// Spawn a child tool entity that produces card content.
		// The render tool calls this child to get the spawned Entity.
		OnSpawn::insert_child({
			let content_func = func.clone();
			(
				CardContentHandler,
				RouteHidden,
				CardContentFn::new(move |world| {
					world.spawn((Card, content_func())).id()
				}),
				ToolMeta::of::<F, (), Entity>(),
				card_content_tool_handler(func),
			)
		}),
		card_tool_handler(),
	)
}

/// Creates the [`ToolHandler<(), Entity>`] that spawns card content
/// by reading the [`CardContentFn`] component and returning the
/// spawned root [`Entity`].
fn card_content_tool_handler<F, B>(_func: F) -> ToolHandler<(), Entity>
where
	F: 'static + Send + Sync + Clone + Fn() -> B,
	B: 'static + Send + Sync + Bundle,
{
	ToolHandler::new(
		move |ToolCall {
		          mut commands,
		          tool,
		          input: (),
		          out_handler,
		      }| {
			commands.commands.queue(move |world: &mut World| -> Result {
				let content_fn = world
					.entity(tool)
					.get::<CardContentFn>()
					.cloned()
					.ok_or_else(|| {
						bevyhow!(
							"CardContentHandler entity missing CardContentFn"
						)
					})?;

				let card_entity = content_fn.spawn(world);

				// Obtain fresh AsyncCommands via SystemState so the
				// out_handler can queue further work.
				let result = {
					let mut state = SystemState::<AsyncCommands>::new(world);
					let async_commands = state.get_mut(world);
					let result = out_handler.call(async_commands, card_entity);
					state.apply(world);
					result
				};
				world.flush();
				result
			});
			Ok(())
		},
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
#[cfg(feature = "markdown")]
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

/// Creates the [`ToolHandler<Request, Response>`] for a card tool
/// entity.
///
/// Finds the nearest [`RenderToolMarker`] by traversing to the root,
/// creates a [`RenderRequest`] with the card's content handler child,
/// and delegates to the render tool for the actual rendering.
fn card_tool_handler() -> ToolHandler<Request, Response> {
	ToolHandler::new(
		move |ToolCall {
		          mut commands,
		          tool: tool_entity,
		          input: request,
		          out_handler,
		      }| {
			commands.commands.queue(move |world: &mut World| -> Result {
				// Find the CardContentHandler child of this card tool entity
				let handler_entity = world
					.run_system_once_with(
						|In(parent): In<Entity>,
						 children_query: Query<&Children>,
						 handler_query: Query<
							Entity,
							With<CardContentHandler>,
						>| {
							if let Ok(children) = children_query.get(parent) {
								children.iter().find(|child| {
									handler_query.contains(*child)
								})
							} else {
								None
							}
						},
						tool_entity,
					)
					.ok()
					.flatten()
					.ok_or_else(|| {
						bevyhow!(
							"Card tool entity missing CardContentHandler child"
						)
					})?;

				// Find the render tool by traversal
				let render_tool = find_render_tool(world, tool_entity)?;

				// Use AsyncCommands to spawn the async task
				let mut state = SystemState::<AsyncCommands>::new(world);
				let mut async_commands = state.get_mut(world);

				async_commands.run(async move |world: AsyncWorld| -> Result {
					let render_request = RenderRequest {
						handler: handler_entity,
						discover_call: false,
						request,
					};

					// Call the render tool
					let response: Response = world
						.entity(render_tool)
						.call::<RenderRequest, Response>(render_request)
						.await?;

					// Deliver response via the out handler using fresh
					// AsyncCommands obtained through SystemState
					world
						.with_then(move |world: &mut World| -> Result {
							let result = {
								let mut state =
									SystemState::<AsyncCommands>::new(world);
								let async_commands = state.get_mut(world);
								let result =
									out_handler.call(async_commands, response);
								state.apply(world);
								result
							};
							world.flush();
							result
						})
						.await
				});

				state.apply(world);
				world.flush();
				Ok(())
			});
			Ok(())
		},
	)
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
}
