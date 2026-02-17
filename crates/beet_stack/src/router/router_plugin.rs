use crate::prelude::*;
use beet_core::prelude::*;
use std::collections::VecDeque;

/// Plugin that registers route-building observers for tools.
///
/// Automatically constructs a [`RouteTree`] on the root ancestor
/// whenever tools are spawned in an entity hierarchy. Cards now
/// register as tools (via [`Card`] + [`ToolMeta`]), so there
/// is no separate card observer.
#[derive(Default)]
pub struct RouterPlugin;

impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(insert_tool_path_and_params)
			.add_observer(insert_route_tree);
	}
}

/// Observer that listens for new tools and inserts their path and params patterns.
/// Any [`PathPartial`] or [`ParamsPartial`] will be collected so long as they are
/// spawned at the same time as the tool, even if they come after it in the tuple.
/// This is because, unlike OnAdd component hooks, observers run after the entire
/// tree is spawned.
pub fn insert_tool_path_and_params(
	ev: On<Insert, ToolMeta>,
	ancestors: Query<&ChildOf>,
	paths: Query<&PathPartial>,
	params: Query<&ParamsPartial>,
	mut commands: Commands,
) -> Result {
	insert_path_and_params(
		ev.entity,
		&ancestors,
		&paths,
		&params,
		&mut commands,
	)
}

/// Shared logic for collecting ancestor path and param partials
/// and inserting the resolved [`PathPattern`] and [`ParamsPattern`]
/// on the given entity.
fn insert_path_and_params(
	entity: Entity,
	ancestors: &Query<&ChildOf>,
	paths: &Query<&PathPartial>,
	params: &Query<&ParamsPartial>,
	commands: &mut Commands,
) -> Result {
	let path = PathPattern::collect(entity, ancestors, paths)?;
	let params = ParamsPattern::collect(entity, ancestors, params)?;
	commands.entity(entity).insert((path, params));
	Ok(())
}

/// Observer that rebuilds the [`RouteTree`] on the root ancestor
/// whenever a [`PathPattern`] is inserted on any entity in the hierarchy.
///
/// Collects all entities with tool components ([`ToolMeta`], [`PathPattern`],
/// [`ParamsPattern`]) from the root's descendants and constructs a validated
/// tree. Cards are distinguished from regular tools by the presence of a
/// [`Card`] component, which sets `is_card: true` on the [`ToolNode`].
pub fn insert_route_tree(
	ev: On<Insert, PathPattern>,
	ancestors: Query<&ChildOf>,
	children_query: Query<&Children>,
	tools: Query<
		(
			Entity,
			&ToolMeta,
			&PathPattern,
			&ParamsPattern,
			Option<&HttpMethod>,
			Option<&Card>,
		),
		Without<RouteHidden>,
	>,
	mut commands: Commands,
) -> Result {
	let root = ancestors.root_ancestor(ev.entity);
	let mut nodes: Vec<ToolNode> = Vec::new();

	// Check if any card entities have CardContentFn children
	let has_discoverable_cards = children_query
		.iter_descendants_inclusive(root)
		.any(|entity| {
			tools
				.get(entity)
				.map(|(_, _, _, _, _, card)| card.is_some())
				.unwrap_or(false)
		});

	for entity in children_query.iter_descendants_inclusive(root) {
		if let Ok(item) = tools.get(entity) {
			nodes.push(ToolNode::from_query(item));
		}
	}

	if nodes.is_empty() {
		return Ok(());
	}

	let tree = RouteTree::from_nodes(nodes)?;
	commands.entity(root).insert(tree);

	// If there are cards, queue discovery to find nested tools
	// inside card content that aren't visible in the static hierarchy.
	if has_discoverable_cards {
		commands.queue(move |world: &mut World| {
			if let Err(err) = discover_card_routes(world, root) {
				warn!("Card route discovery failed: {err}");
			}
		});
	}

	Ok(())
}

/// Discovers nested tools inside card content by temporarily
/// spawning each card's content, collecting routes, and despawning.
///
/// Cards may contain nested tools or even nested cards. This
/// function spawns each card's content via [`CardContentFn`],
/// collects any [`ToolMeta`] + [`PathPattern`] entities from the
/// spawned tree, adds them to the existing [`RouteTree`], and
/// recursively discovers nested cards.
fn discover_card_routes(world: &mut World, root: Entity) -> Result {
	// Collect card content handlers that have a CardContentFn
	let card_handlers: Vec<(Entity, CardContentFn)> = world
		.query_filtered::<(Entity, &CardContentFn), Without<RouteHidden>>()
		.iter(world)
		.filter(|(entity, _)| {
			// Only discover handlers that are descendants of this root
			let mut current = *entity;
			loop {
				if let Some(child_of) = world.entity(current).get::<ChildOf>() {
					current = child_of.parent();
				} else {
					break;
				}
			}
			current == root
		})
		.map(|(entity, content_fn)| (entity, content_fn.clone()))
		.collect();

	if card_handlers.is_empty() {
		return Ok(());
	}

	let mut discovered_nodes: Vec<ToolNode> = Vec::new();

	for (_handler_entity, content_fn) in &card_handlers {
		// Spawn the card content temporarily
		let content_entity = content_fn.spawn(world);

		// Flush to ensure observers (PathPattern insertion etc) run
		world.flush();

		// Collect tool nodes from the spawned content tree
		let mut queue = VecDeque::new();
		queue.push_back(content_entity);
		while let Some(entity) = queue.pop_front() {
			// Check for tool components (non-hidden)
			let has_tool = world.entity(entity).contains::<ToolMeta>()
				&& world.entity(entity).contains::<PathPattern>()
				&& !world.entity(entity).contains::<RouteHidden>();

			if has_tool {
				let meta =
					world.entity(entity).get::<ToolMeta>().unwrap().clone();
				let path =
					world.entity(entity).get::<PathPattern>().unwrap().clone();
				let params = world
					.entity(entity)
					.get::<ParamsPattern>()
					.cloned()
					.unwrap_or_default();
				let method = world.entity(entity).get::<HttpMethod>().cloned();
				let is_card = world.entity(entity).contains::<Card>();

				discovered_nodes.push(ToolNode {
					entity,
					meta,
					params,
					path,
					method,
					is_card,
				});
			}

			// Traverse children
			if let Some(children) = world.entity(entity).get::<Children>() {
				for child in children.iter() {
					queue.push_back(child);
				}
			}
		}

		// Despawn the temporary content
		world.entity_mut(content_entity).despawn();
	}

	if discovered_nodes.is_empty() {
		return Ok(());
	}

	// Merge discovered nodes with the existing tree
	let existing_tree = world.entity(root).get::<RouteTree>();
	let mut all_nodes = if let Some(tree) = existing_tree {
		tree.flatten_nodes()
			.into_iter()
			.cloned()
			.collect::<Vec<_>>()
	} else {
		Vec::new()
	};

	// Deduplicate by entity — discovered nodes may overlap with
	// existing ones if observers already inserted them.
	for node in discovered_nodes {
		if !all_nodes
			.iter()
			.any(|existing| existing.entity == node.entity)
		{
			all_nodes.push(node);
		}
	}

	match RouteTree::from_nodes(all_nodes) {
		Ok(tree) => {
			world.entity_mut(root).insert(tree);
		}
		Err(err) => {
			warn!("Failed to rebuild route tree with discovered cards: {err}");
		}
	}

	Ok(())
}
