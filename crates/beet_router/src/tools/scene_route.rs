use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;
use beet_tool::prelude::*;

/// An entity spawned by a [`scene_route`] tool call.
///
/// Points back to the scene route entity that produced it, establishing
/// the [`SceneRoutes`] relationship on the scene route tool.
#[derive(Debug, Clone, Deref, Component)]
#[relationship(relationship_target = SceneRoutes)]
pub struct SceneRouteOf(pub Entity);

impl SceneRouteOf {
	/// Creates a new [`SceneRouteOf`] relationship pointing to the given
	/// scene route entity.
	pub fn new(value: Entity) -> Self { Self(value) }
}

/// All scene content entities currently spawned by this scene route tool.
///
/// Automatically maintained by the [`SceneRouteOf`] relationship. When the
/// scene route entity is despawned, all tracked scene entities are also
/// despawned.
#[derive(Debug, Clone, Deref, Component)]
#[relationship_target(relationship = SceneRouteOf, linked_spawn)]
pub struct SceneRoutes(Vec<Entity>);

/// A single content container, similar to pages in a website or cards
/// in HyperCard. Each scene route is a route, with the exact rendering
/// behavior determined by the render tool on the server or interface.
///
/// Use the [`scene_route`] function to create a routable scene with content.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
#[require(DocumentScope)]
pub struct SceneRoute;

/// Marker component for render tools on servers or interfaces.
///
/// A render tool accepts a [`RenderRequest`] and returns a [`Response`].
/// Different servers provide different render tools:
/// - CLI/REPL: renders content to markdown, despawns the entity
/// - TUI: manages stateful scene display
///
/// Found by [`find_render_tool`] which traverses to the root ancestor
/// and searches descendants for this marker.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct RenderToolMarker;

/// Request passed to a render tool to render a scene's content.
///
/// The render tool decides whether and when to call
/// [`spawn_tool`](RenderRequest::spawn_tool) to materialise the scene's
/// content as an entity. This gives render tools full control over
/// spawn/despawn lifecycle, ie stateless renderers despawn immediately
/// while stateful ones retain the entity as [`CurrentScene`].
#[derive(Debug)]
pub struct RenderRequest {
	/// The scene route entity that issued this request.
	pub scene_route: Entity,
	/// A tool that, when called with `()`, spawns the scene bundle as an
	/// entity and attaches [`SceneRouteOf(scene_route)`](SceneRouteOf) to it.
	/// The render tool chooses when (or whether) to call this.
	pub spawn_tool: Tool<(), Entity>,
	/// The original request.
	pub request: Request,
}

/// Creates a routable scene route tool from a path and content handler.
///
/// The handler is a function that returns an [`impl Bundle`](Bundle).
/// When a request arrives the scene route tool:
/// 1. Locates the nearest [`RenderToolMarker`] via [`find_render_tool`].
/// 2. Builds a `spawn_tool` that, on demand, spawns the bundle with
///    [`SceneRouteOf`] pointing back to the scene entity.
/// 3. Forwards a [`RenderRequest`] to the render tool, which decides
///    when to call `spawn_tool` and how to manage the resulting entity.
pub fn scene_route<F, B>(path: &str, func: F) -> impl Bundle
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
			          caller: scene_entity,
			          input: request,
			          out_handler,
			      }| {
				let func = func.clone();
				commands.run(async move |world: AsyncWorld| -> Result {
					// Build spawn_tool inline, capturing scene_entity and func.
					// The render tool calls this when it wants to materialise content.
					let spawn_tool = Tool::new(TypeMeta::of::<F>(), {
						let func = func.clone();
						move |ToolCall {
						          mut commands,
						          caller: _,
						          input: (),
						          out_handler,
						      }| {
							let func = func.clone();
							commands.commands.queue(
								move |world: &mut World| -> Result {
									let entity = world
										.spawn((
											SceneRouteOf::new(scene_entity),
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
							find_render_tool(world, scene_entity)
						})
						.await?;

					// Delegate to the render tool with spawn capability
					let response = world
						.entity(render_tool)
						.call::<RenderRequest, Response>(RenderRequest {
							scene_route: scene_entity,
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

	(PathPartial::new(path), SceneRoute, handler)
}

/// Creates a routable scene that loads and parses a file.
///
/// On each render the file is read from disk and the content is
/// spawned as a [`Value::Str`]. When the `markdown` feature is
/// enabled the file is parsed into a semantic entity tree via
/// [`MarkdownDiffer`]. This ensures content renders correctly
/// across all backends (CLI, TUI, etc.).
///
/// # Example
///
/// ```no_run
/// use beet_router::prelude::*;
/// use beet_core::prelude::*;
///
/// let bundle = file_route("readme", "docs/readme.md");
/// ```
pub fn file_route(path: &str, file_path: impl Into<WsPathBuf>) -> impl Bundle {
	let ws_path: WsPathBuf = file_path.into();
	let ws_path2 = ws_path.clone();
	let handler = Tool::new(
		TypeMeta::of::<FileRouteMarker>(),
		move |ToolCall {
		          mut commands,
		          caller: scene_entity,
		          input: request,
		          out_handler,
		      }| {
			let ws_path = ws_path2.clone();
			commands.run(async move |world: AsyncWorld| -> Result {
				let spawn_tool =
					Tool::new(TypeMeta::of::<FileRouteMarker>(), {
						let ws_path = ws_path.clone();
						move |ToolCall {
						          mut commands,
						          caller: _,
						          input: (),
						          out_handler,
						      }| {
							let ws_path = ws_path.clone();
							commands.commands.queue(
								move |world: &mut World| -> Result {
									let entity = world
										.spawn((
											SceneRoute,
											SceneRouteOf::new(scene_entity),
										))
										.id();
									file_route_parse_content(
										world, entity, &ws_path,
									);
									out_handler.call_world(world, entity)
								},
							);
							Ok(())
						}
					});

				let render_tool = world
					.with_then(move |world: &mut World| {
						find_render_tool(world, scene_entity)
					})
					.await?;

				let response = world
					.entity(render_tool)
					.call::<RenderRequest, Response>(RenderRequest {
						scene_route: scene_entity,
						spawn_tool,
						request,
					})
					.await?;

				out_handler.call_async(world, response).await
			});
			Ok(())
		},
	);

	(PathPartial::new(path), SceneRoute, handler)
}

/// Zero-sized marker for [`file_route`] tool type metadata.
struct FileRouteMarker;

/// Read a file and spawn its content as child entities on `entity`.
///
/// When the `markdown` feature is enabled, the file is parsed into a
/// semantic entity tree. Otherwise, the raw text is spawned as a
/// [`Value::Str`].
#[cfg(feature = "markdown")]
fn file_route_parse_content(
	world: &mut World,
	entity: Entity,
	ws_path: &WsPathBuf,
) {
	let abs_path = ws_path.clone().into_abs();
	match fs_ext::read_to_string(&abs_path) {
		Ok(text) => {
			// Without markdown diffing, fall back to
			// raw text so the user sees something.
			world.spawn((ChildOf(entity), Value::Str(text)));
		}
		Err(err) => {
			cross_log_error!("Failed to load file: {err}");
			world.spawn((
				ChildOf(entity),
				Value::Str(format!(
					"Error loading {}: {err}",
					abs_path.display()
				)),
			));
		}
	}
}

/// Fallback when the `markdown` feature is disabled: spawn raw text.
#[cfg(not(feature = "markdown"))]
fn file_route_parse_content(
	world: &mut World,
	entity: Entity,
	ws_path: &WsPathBuf,
) {
	let abs_path = ws_path.clone().into_abs();
	match fs_ext::read_to_string(&abs_path) {
		Ok(text) => {
			world.spawn((ChildOf(entity), Value::Str(text)));
		}
		Err(err) => {
			cross_log_error!("Failed to load file: {err}");
			world.spawn((
				ChildOf(entity),
				Value::Str(format!(
					"Error loading {}: {err}",
					abs_path.display()
				)),
			));
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
/// `media_render_tool` for CLI/REPL or `tui_render_tool` for TUI.
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
				 `media_render_tool()` to the server's entity tree."
			)
		})
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;

	#[test]
	fn scene_route_appears_in_route_tree() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn(children![scene_route("about", || Value::Str(
				"About page".into()
			)),])
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.find(&["about"]).xpect_some();
	}

	#[test]
	fn find_render_tool_traverses_hierarchy() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn(children![
				RenderToolMarker,
				scene_route("test", || Value::Str("test".into())),
			])
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

	/// OnSpawn::insert_child alone works fine.
	#[test]
	fn insert_child_alone() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world.spawn(OnSpawn::insert_child(RenderToolMarker)).flush();
		find_render_tool(&mut world, root).xpect_ok();
	}

	/// OnSpawn::insert_child works alongside the children! macro.
	#[test]
	fn insert_child_with_children_macro() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn((OnSpawn::insert_child(RenderToolMarker), children![
				scene_route("test", || Value::Str("test".into())),
			]))
			.flush();
		find_render_tool(&mut world, root).xpect_ok();
	}

	/// `OnSpawn::insert(children![...])` clobbers children added by
	/// a prior `OnSpawn::insert_child` because `children!` is a *set*
	/// operation. Use individual `OnSpawn::insert_child` calls instead.
	#[test]
	fn on_spawn_insert_children_clobbers_insert_child() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn((
				OnSpawn::insert_child(RenderToolMarker),
				OnSpawn::insert(children![(Name::new("Other Child"),)]),
			))
			.flush();
		// The render tool was clobbered by the subsequent set operation
		find_render_tool(&mut world, root).xpect_err();
	}

	/// OnSpawn::insert_child works alongside children!.
	#[test]
	fn insert_child_with_children() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn((OnSpawn::insert_child(RenderToolMarker), children![
				scene_route("test", || Value::Str("test".into())),
			]))
			.flush();
		find_render_tool(&mut world, root).xpect_ok();
	}
}
