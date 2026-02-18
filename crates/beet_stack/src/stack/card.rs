use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemState;

/// A single content container, similar to pages in a website or cards
/// in HyperCard. Each card is a route, with the exact rendering behavior
/// determined by the render tool on the server or interface.
///
/// Cards are tools that delegate rendering to a [`RenderToolMarker`]
/// entity found by traversing to the root ancestor. The `Card` component
/// serves both as a boundary marker for
/// [`CardWalker`](crate::utils::CardWalker) and [`DocumentQuery`]
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
/// The card tool spawns content via its child content handler
/// and passes the resulting entity to the render tool. The render
/// tool is responsible for rendering and lifecycle management
/// (eg despawning for stateless renderers, or marking with
/// [`CurrentCard`] for stateful ones).
#[derive(Debug)]
pub struct RenderRequest {
	/// The spawned card content entity.
	pub entity: Entity,
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
/// Internally, the handler is converted to a `ToolHandler<(), Entity>`
/// child tool that spawns the bundle and returns the entity. The card
/// tool then pipes the entity to the render tool.
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
	let content_handler = {
		let func = func.clone();
		ToolHandler::new(
			move |ToolCall {
			          mut commands,
			          tool: _tool,
			          input: (),
			          out_handler,
			      }| {
				let func = func.clone();
				commands.commands.queue(move |world: &mut World| -> Result {
					let entity = world.spawn((Card, func())).id();
					let result = {
						let mut state =
							SystemState::<AsyncCommands>::new(world);
						let async_commands = state.get_mut(world);
						let result = out_handler.call(async_commands, entity);
						state.apply(world);
						result
					};
					world.flush();
					result
				});
				Ok(())
			},
		)
	};

	(
		PathPartial::new(path),
		Card,
		// Outer tool meta: the card presents as Request/Response
		ToolMeta::of::<F, Request, Response>(),
		// Spawn a child tool entity that produces card content.
		// The card tool calls this child to spawn content and get the Entity.
		OnSpawn::insert_child((
			RouteHidden,
			ToolMeta::of::<F, (), Entity>(),
			content_handler,
		)),
		card_tool_handler(),
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

/// Creates the [`ToolHandler<Request, Response>`] for a card tool entity.
///
/// When called:
/// 1. Calls the first child entity (content handler) to spawn content
/// 2. Finds the nearest [`RenderToolMarker`] via ancestor traversal
/// 3. Passes the spawned entity and request to the render tool
/// 4. Returns the render tool's response
fn card_tool_handler() -> ToolHandler<Request, Response> {
	ToolHandler::new(
		move |ToolCall {
		          mut commands,
		          tool: tool_entity,
		          input: request,
		          out_handler,
		      }| {
			commands.run(async move |world: AsyncWorld| -> Result {
				// Get the first child entity (content handler)
				let child = world
					.entity(tool_entity)
					.get(|children: &Children| children[0])
					.await?;

				// Call content handler to spawn content
				let card_entity: Entity = world.entity(child).call(()).await?;

				// Find the render tool by traversal
				let render_tool = world
					.with_then(move |world: &mut World| {
						find_render_tool(world, tool_entity)
					})
					.await?;

				// Call the render tool with the spawned entity
				let response: Response = world
					.entity(render_tool)
					.call::<RenderRequest, Response>(RenderRequest {
						entity: card_entity,
						request,
					})
					.await?;

				// Deliver response
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
