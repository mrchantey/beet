use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;

/// A single content container, similar to pages in a website or cards
/// in HyperCard. Each card is a route, with the exact rendering behavior
/// determined by the render tool on the server or interface.
///
/// Cards are tools that delegate rendering to a [`RenderToolMarker`]
/// entity found by traversing to the root ancestor. The `Card` component
/// itself serves as a boundary marker for [`CardWalker`](crate::renderers::CardWalker)
/// and [`DocumentQuery`] traversal.
///
/// Use the [`card`] function to create a routable card with content:
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = StackPlugin::world();
/// let root = world.spawn((
///     default_interface(),
///     children![
///         card("about", || Paragraph::with_text("About page")),
///     ],
/// )).flush();
///
/// let tree = world.entity(root).get::<RouteTree>().unwrap();
/// tree.find(&["about"]).xpect_some();
/// ```
#[derive(Component)]
pub struct Card;

/// Marker component on tool entities that are cards, distinguishing
/// them from regular tools in help display and route tree queries.
///
/// Added automatically by the [`card`] function. Unlike the [`Card`]
/// component (which marks content boundaries), this marker lives on
/// the route tool entity in the tree.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct CardMarker;

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
/// The render tool decides how and when to call the [`CardSpawner`]
/// on the handler entity, enabling stateful renderers (like TUI) to
/// reuse existing card entities instead of respawning on every request.
#[derive(Debug)]
pub struct RenderRequest {
	/// The card tool entity that has a [`CardSpawner`] component.
	/// The render tool reads the spawner to create the card content.
	pub handler: Entity,
	/// Cards must be called once at first to discover their
	/// nested tools. When true, the render tool should avoid
	/// unnecessary on-mount work.
	pub discover_call: bool,
	/// The original request.
	pub request: Request,
}

/// Type-erased card content spawner stored on the card tool entity.
///
/// Contains a boxed function that spawns card content into the world
/// and returns the root [`Entity`] of the spawned content tree.
/// The spawned entity always gets a [`Card`] marker for walker
/// boundary detection.
#[derive(Component, Clone)]
pub struct CardSpawner(Arc<dyn Fn(&mut World) -> Entity + Send + Sync>);

impl CardSpawner {
	/// Create a new spawner from a function.
	pub fn new(
		func: impl Fn(&mut World) -> Entity + 'static + Send + Sync,
	) -> Self {
		Self(Arc::new(func))
	}

	/// Spawn the card content into the world, returning the root entity.
	pub fn spawn(&self, world: &mut World) -> Entity { (self.0)(world) }
}

impl std::fmt::Debug for CardSpawner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CardSpawner").finish_non_exhaustive()
	}
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
///     default_interface(),
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
	F: 'static + Send + Sync + Fn() -> B,
	B: 'static + Send + Sync + Bundle,
{
	(
		PathPartial::new(path),
		CardMarker,
		// Outer tool meta: the card presents as Request/Response
		ToolMeta::of::<F, Request, Response>(),
		CardSpawner::new(move |world| world.spawn((Card, func())).id()),
		OnSpawn::observe(card_tool_handler),
	)
}

/// Creates a routable card that loads its content from a markdown file.
///
/// On each request, the file is read from disk and parsed as markdown.
/// This replaces the [`FileContent`] paradigm with a proper tool-based
/// approach.
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
	(
		PathPartial::new(path),
		CardMarker,
		ToolMeta::of::<fn(), Request, Response>(),
		CardSpawner::new(move |world| {
			let abs_path = ws_path.clone().into_abs();
			let entity = world.spawn(Card).id();
			match fs_ext::read_to_string(&abs_path) {
				Ok(text) => {
					let mut differ = crate::parsers::MarkdownDiffer::new(&text);
					if let Err(err) = differ.diff(world.entity_mut(entity)) {
						cross_log_error!(
							"Failed to parse markdown file: {err}"
						);
					}
				}
				Err(err) => {
					cross_log_error!("Failed to load file: {err}");
				}
			}
			entity
		}),
		OnSpawn::observe(card_tool_handler),
	)
}

/// Observer that handles incoming [`Request`] on a card tool entity.
///
/// Finds the nearest [`RenderToolMarker`] by traversing to the root,
/// creates a [`RenderRequest`] with the card entity as handler,
/// and delegates to the render tool for the actual rendering.
fn card_tool_handler(
	mut ev: On<ToolIn<Request, Response>>,
	mut commands: AsyncCommands,
) -> Result {
	let ev = ev.event_mut();
	let tool_entity = ev.tool();
	let request = ev.take_input()?;
	let outer_handler = ev.take_out_handler()?;

	commands.run(async move |mut world| -> Result {
		// Find the render tool by traversal
		let render_tool = world
			.with_then(move |world: &mut World| -> Result<Entity> {
				find_render_tool(world, tool_entity)
			})
			.await?;

		let render_request = RenderRequest {
			handler: tool_entity,
			discover_call: false,
			request,
		};

		// Call the render tool
		let response: Response = world
			.entity(render_tool)
			.call::<RenderRequest, Response>(render_request)
			.await?;

		outer_handler.call_async(&mut world, tool_entity, response)
	});
	Ok(())
}

/// Finds the nearest render tool by traversing to the root ancestor
/// and searching descendants for a [`RenderToolMarker`].
///
/// # Errors
///
/// Returns an error if no render tool is found in the hierarchy.
/// Ensure a render tool like [`markdown_render_tool`] is added to
/// the interface or server.
pub fn find_render_tool(world: &World, entity: Entity) -> Result<Entity> {
	// Walk to root
	let mut current = entity;
	while let Some(child_of) = world.entity(current).get::<ChildOf>() {
		current = child_of.parent();
	}
	let root = current;

	// DFS from root for RenderToolMarker
	fn dfs(world: &World, entity: Entity) -> Option<Entity> {
		if world.entity(entity).contains::<RenderToolMarker>() {
			return Some(entity);
		}
		if let Some(children) = world.entity(entity).get::<Children>() {
			for child in children.iter() {
				if let Some(found) = dfs(world, child) {
					return Some(found);
				}
			}
		}
		None
	}

	dfs(world, root).ok_or_else(|| {
		bevyhow!(
			"No render tool found. Add a render tool like \
			 `markdown_render_tool()` to the interface or server."
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
			.spawn((default_interface(), children![card("about", || {
				Paragraph::with_text("About page")
			})]))
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
			.spawn((default_interface(), children![card("home", || {
				children![
					Heading1::with_text("Welcome"),
					Paragraph::with_text("Hello!"),
				]
			})]))
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
			.spawn((default_interface(), children![card("about", || {
				Paragraph::with_text("About")
			})]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.find(&["about"]).xpect_some();
	}

	#[test]
	fn find_render_tool_traverses_hierarchy() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((default_interface(), children![card("test", || {
				Paragraph::with_text("test")
			})]))
			.flush();

		let result = find_render_tool(&world, root);
		result.xpect_ok();
	}

	#[test]
	fn find_render_tool_errors_without_render_tool() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		find_render_tool(&world, entity).xpect_err();
	}
}
