use crate::prelude::*;
use alloc::collections::VecDeque;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Marker component that excludes an entity from the [`RouteTree`].
///
/// Internal actions like fallback chain handlers should not appear
/// as routable endpoints. Adding this component prevents them from
/// being collected during route tree construction.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct RouteHidden;

/// Marks a route that renders a user-facing page, the routes that populate the
/// navigation [`RouteSidebar`](crate::prelude::RouteSidebar).
///
/// Auto-attached by codegen route emission (`emit_routes`) to `.rs` page routes
/// and markdown/blob content routes. Infrastructure routes injected by
/// [`Router::with_defaults`](crate::prelude::default_router) (`app_info`, `analytics`,
/// the `js/reactivity.js` asset, `client_io`) are not codegen-emitted, so they
/// never carry it and stay out of the nav.
///
/// Distinct from [`PageRequest`]: a `PageRoute` marks a route entity, while a
/// [`PageRequest`] is the render handle such a route produces per request.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
pub struct PageRoute;

/// Collects all routes (actions and scene routes) in an entity hierarchy and
/// arranges them into a validated tree.
///
/// Inserted on the root ancestor whenever a [`PathPattern`] is set.
/// Ensures there is only a single route for any given path pattern and
/// detects conflicts between dynamic and greedy segments that would
/// cause ambiguous routing.
///
/// ## Validation Rules
/// - Only one route per exact path pattern
/// - Cannot mix static and dynamic segments at the same level
/// - Cannot have multiple dynamic segments at the same level
/// - Greedy segments must be the last segment in a path
#[derive(Debug, Clone, Component)]
pub struct RouteTree {
	/// The path pattern for this tree node.
	pub path: PathPattern,
	/// The params pattern for this tree node.
	pub params: ParamsPattern,
	/// The route at this exact path, if any.
	node: Option<ActionNode>,
	/// Child nodes in the tree.
	pub children: Vec<RouteTree>,
}

impl RouteTree {
	/// Returns the [`ActionNode`] at this level of the tree, if any.
	pub fn node(&self) -> Option<&ActionNode> { self.node.as_ref() }

	/// Builds a [`RouteTree`] from a list of [`ActionNode`].
	///
	/// ## Errors
	///
	/// Returns an error if there are conflicting or duplicate paths.
	pub fn from_nodes(nodes: Vec<ActionNode>) -> Result<Self> {
		#[derive(Default)]
		struct Node {
			children: HashMap<String, Node>,
			route: Option<ActionNode>,
			params: Option<ParamsPattern>,
			is_static: Option<bool>,
		}

		let mut root = Node::default();

		for action_node in &nodes {
			let path = &action_node.path;
			let segments = path.iter().cloned().collect::<Vec<_>>();
			let mut node = &mut root;

			for (idx, seg) in segments.iter().enumerate() {
				let is_last = idx == segments.len() - 1;
				let seg_is_static = seg.is_static();
				let key = seg.to_string_annotated();

				// check for conflicts at this level
				for (existing_key, existing_node) in &node.children {
					let existing_is_static =
						existing_node.is_static.unwrap_or(true);

					if existing_key != &key
						&& !seg_is_static && !existing_is_static
					{
						bevybail!(
							"Path conflict: cannot have multiple dynamic/greedy segments at same level. \
							Found '{}' and '{}' at the same position",
							existing_key,
							key
						);
					}

					if existing_key != &key
						&& (seg_is_static != existing_is_static)
					{
						bevybail!(
							"Path conflict: cannot mix static and dynamic segments at same level. \
							Found '{}' and '{}'",
							existing_key,
							key
						);
					}
				}

				node = node.children.entry(key).or_insert_with(|| Node {
					is_static: Some(seg_is_static),
					route: None,
					params: None,
					children: default(),
				});

				if is_last {
					if node.route.is_some() {
						bevybail!(
							"Duplicate route: multiple routes defined for path '{}'",
							path.annotated_path()
						);
					}
					node.route = Some(action_node.clone());
					node.params = Some(action_node.params.clone());
				}
			}

			// handle root path (empty segments)
			if segments.is_empty() {
				if node.route.is_some() {
					bevybail!(
						"Duplicate route: multiple routes defined for path '/'"
					);
				}
				node.route = Some(action_node.clone());
				node.params = Some(action_node.params.clone());
			}
		}

		/// Recursively build the RouteTree, sorting children by their path.
		fn build_tree(
			pattern: PathPattern,
			params: ParamsPattern,
			node: &Node,
		) -> RouteTree {
			let mut children: Vec<RouteTree> = node
				.children
				.iter()
				.map(|(key, child_node)| {
					let segment = PathPatternSegment::new(key);
					let mut child_segments =
						pattern.iter().cloned().collect::<Vec<_>>();
					child_segments.push(segment);
					let child_pattern =
						PathPattern::from_segments(child_segments).unwrap();
					let child_params =
						child_node.params.clone().unwrap_or(params.clone());
					build_tree(child_pattern, child_params, child_node)
				})
				.collect();

			children.sort_by(|a, b| a.path.cmp(&b.path));

			RouteTree {
				path: pattern,
				params: node.params.clone().unwrap_or(params),
				node: node.route.clone(),
				children,
			}
		}

		build_tree(
			PathPattern::from_segments(vec![]).unwrap(),
			ParamsPattern::default(),
			&root,
		)
		.xok()
	}

	/// System wrapper around
	/// [`RouteTreeBuilder::rebuild_subtree`](crate::prelude::RouteTreeBuilder::rebuild_subtree),
	/// for callers that only hold a bevy system-running handle rather than a
	/// live [`RouteTreeBuilder`](crate::prelude::RouteTreeBuilder): the scene
	/// reparent and scene-despawn call sites, via
	/// [`run_system_cached_with`](bevy::ecs::world::World::run_system_cached_with).
	pub fn rebuild(
		server: In<Entity>,
		mut builder: RouteTreeBuilder,
	) -> Result {
		builder.rebuild_subtree(*server)
	}

	/// Resolves the [`RouteTree`] governing `entity` from live queries,
	/// returning the entity the winning tree lives on alongside the tree.
	///
	/// A url space at or under `entity` wins over the one hosting it, and among
	/// several the one that [serves pages](Self::serves_pages) wins — that is the
	/// *site* such a caller means (rendering, forwarding a capability call) —
	/// else the first tree found. A server mounted under a command route
	/// (`<Route path="serve" {TuiServer}>`) sits *inside* the command namespace
	/// while serving its own `Router` child, so preferring the enclosing one
	/// would dispatch its pages at the command url space, where no page route
	/// exists. Only when nothing beneath carries a tree does `entity` sit inside
	/// a url space rather than above one, and its enclosing namespace
	/// ([`PathPattern::namespace_root`]) answers.
	///
	/// Lives in the no_std core (rather than as a method on the std-only
	/// `RouteQuery`) so [`find_router`](crate::prelude::find_router), also
	/// no_std, can share it; `RouteQuery::resolve_tree` is a thin wrapper
	/// binding a query's own fields, and [`Self::of`] is the `&World`
	/// counterpart for a caller holding a `World` rather than live queries.
	///
	/// # Errors
	/// Errors when nothing at or under `entity`'s namespace carries a tree.
	pub fn resolve<'a>(
		entity: Entity,
		ancestors: &Query<&ChildOf>,
		paths: &Query<&PathPartial>,
		children_query: &Query<&Children>,
		trees: &'a Query<&RouteTree>,
	) -> Result<(Entity, &'a RouteTree)> {
		// every url space at or under `entity`, nested ones included: a command
		// dispatcher hosts both its own commands and the site under `serve`.
		let mut candidates: Vec<(Entity, &RouteTree)> = Vec::new();
		let mut queue = vec![entity];
		while let Some(entity) = queue.pop() {
			if let Ok(tree) = trees.get(entity) {
				candidates.push((entity, tree));
			}
			if let Ok(children) = children_query.get(entity) {
				queue.extend(children.iter());
			}
		}
		candidates
			.iter()
			.find(|(_, tree)| tree.serves_pages())
			.or(candidates.first())
			.copied()
			// nothing beneath: `entity` sits inside a url space rather than above
			// one, so the space it belongs to is the answer.
			.or_else(|| {
				let near =
					PathPattern::namespace_root(entity, ancestors, paths);
				trees.get(near).ok().map(|tree| (near, tree))
			})
			.ok_or_else(|| bevyhow!("no RouteTree at or under {entity}"))
	}

	/// The [`RouteTree`] at or under `entity`.
	///
	/// A tree lives on its url space's root, ie the `Router` that dispatches it,
	/// and a built entry root is usually its *server* with the router as a child.
	/// A caller holding the root resolves the tree here rather than assuming
	/// which entity carries it.
	///
	/// An entry can hold several url spaces (a command dispatcher whose site is
	/// one of its routes), so the one that serves PAGES wins: that is the site a
	/// caller holding the entry root means (rendering, checking, exporting).
	/// Absent any, the first tree found.
	///
	/// The `&World` counterpart of [`Self::resolve`] (used from a caller
	/// holding a `World` rather than live `Query`s): both implement the same
	/// "prefer serving pages" pick, just over different traversal primitives.
	///
	/// # Errors
	/// Errors when nothing at or under `entity` carries a tree.
	pub fn of(world: &World, entity: Entity) -> Result<&RouteTree> {
		let mut trees = Vec::new();
		let mut queue = vec![entity];
		while let Some(entity) = queue.pop() {
			let entity = world.entity(entity);
			if let Some(tree) = entity.get::<RouteTree>() {
				trees.push(tree);
			}
			if let Some(children) = entity.get::<Children>() {
				queue.extend(children.iter());
			}
		}
		trees
			.iter()
			.find(|tree| tree.serves_pages())
			.or(trees.first())
			.copied()
			.ok_or_else(|| bevyhow!("no RouteTree at or under {entity}"))
	}

	/// Whether any node in this tree renders a page, ie whether this url space
	/// is a *site* rather than a set of commands or an api.
	pub fn serves_pages(&self) -> bool {
		self.iter_dfs()
			.iter()
			.filter_map(|tree| tree.node.as_ref())
			.any(|node| node.is_page_route)
	}

	/// All nodes of the tree in depth-first pre-order (reading order): each node
	/// before its children, children in their sorted order. The natural order
	/// for rendering a route tree as a document.
	pub fn iter_dfs(&self) -> Vec<&RouteTree> {
		let mut nodes = Vec::new();
		fn inner<'a>(nodes: &mut Vec<&'a RouteTree>, tree: &'a RouteTree) {
			nodes.push(tree);
			for child in &tree.children {
				inner(nodes, child);
			}
		}
		inner(&mut nodes, self);
		nodes
	}

	/// All nodes of the tree in breadth-first order (level order): every node at
	/// a given depth before any node deeper. The natural order for stepping
	/// siblings before descending.
	pub fn iter_bfs(&self) -> Vec<&RouteTree> {
		let mut nodes = Vec::new();
		let mut queue = VecDeque::from([self]);
		while let Some(tree) = queue.pop_front() {
			nodes.push(tree);
			queue.extend(tree.children.iter());
		}
		nodes
	}

	/// Returns all route paths in the tree as a flat list, in [`iter_dfs`]
	/// (reading) order. Nodes with no matching route are skipped.
	///
	/// [`iter_dfs`]: Self::iter_dfs
	pub fn flatten(&self) -> Vec<PathPattern> {
		self.iter_dfs()
			.into_iter()
			.filter_map(|tree| tree.node.as_ref().map(|_| tree.path.clone()))
			.collect()
	}

	/// Returns all route nodes in the tree as a flat list, in [`iter_dfs`]
	/// (reading) order.
	///
	/// [`iter_dfs`]: Self::iter_dfs
	pub fn flatten_nodes(&self) -> Vec<&ActionNode> {
		self.iter_dfs()
			.into_iter()
			.filter_map(|tree| tree.node.as_ref())
			.collect()
	}

	/// Returns all action nodes in the tree as a flat list, skipping scene route nodes.
	pub fn flatten_action_nodes(&self) -> Vec<&ActionNode> {
		self.flatten_nodes()
			.into_iter()
			.filter(|node| !node.is_scene())
			.collect()
	}

	/// Returns all scene route nodes in the tree as a flat list, skipping non-scene actions.
	pub fn flatten_scene_nodes(&self) -> Vec<&ActionNode> {
		self.flatten_nodes()
			.into_iter()
			.filter(|node| node.is_scene())
			.collect()
	}

	/// Find a route node matching the given path segments.
	///
	/// Walks the tree looking for an exact match against
	/// the provided path. There should never be more than one match
	/// as [`RouteTree::from_nodes`] rejects conflicts.
	pub fn find(&self, path: &[impl AsRef<str>]) -> Option<&ActionNode> {
		let path_vec: Vec<SmolStr> =
			path.iter().map(|s| s.as_ref().into()).collect();

		fn inner<'a>(
			node: &'a RouteTree,
			path: &[SmolStr],
		) -> Option<&'a ActionNode> {
			if let Some(route) = &node.node {
				if route
					.path
					.parse_path(path)
					.map(|matched| matched.exact_match())
					.unwrap_or(false)
				{
					return Some(route);
				}
			}
			for child in &node.children {
				if let Some(found) = inner(child, path) {
					return Some(found);
				}
			}
			None
		}
		inner(self, &path_vec)
	}

	/// Find the subtree rooted at the given path prefix.
	///
	/// Walks the tree children matching each segment of `prefix` in
	/// turn, returning the [`RouteTree`] node at that position.
	///
	/// An example usecase for this
	/// is scoping `--help` output to a specific path prefix.
	///
	/// Static segments are matched first. If no static match is found,
	/// a dynamic segment at the same level is used as a fallback.
	///
	/// Returns `None` if no tree node matches the prefix.
	pub fn find_subtree(
		&self,
		prefix: &[impl AsRef<str>],
	) -> Option<&RouteTree> {
		let mut current = self;
		for segment in prefix {
			let seg = segment.as_ref();
			// try static match first
			let matched = current.children.iter().find(|child| {
				child
					.path
					.iter()
					.last()
					.map(|last| last.is_static() && last.name() == seg)
					.unwrap_or(false)
			});
			// fall back to first dynamic match
			let matched = matched.or_else(|| {
				current.children.iter().find(|child| {
					child
						.path
						.iter()
						.last()
						.map(|last| !last.is_static())
						.unwrap_or(false)
				})
			});
			current = matched?;
		}
		Some(current)
	}
}

impl core::fmt::Display for RouteTree {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		fn inner(
			node: &RouteTree,
			f: &mut core::fmt::Formatter<'_>,
		) -> core::fmt::Result {
			if let Some(action) = &node.node {
				let path = node.path.annotated_path();
				if action.is_scene() {
					writeln!(f, "  {} [scene]", path)?;
				} else {
					let input = action.meta.input().type_name();
					let output = action.meta.output().type_name();
					write!(f, "  {}", path)?;
					if let Some(method) = &action.method {
						write!(f, " [{}]", method)?;
					}
					writeln!(f)?;
					writeln!(f, "    input:  {}", input)?;
					writeln!(f, "    output: {}", output)?;
				}
				for param in node.params.iter() {
					writeln!(f, "    {}", param)?;
				}
			}
			for child in &node.children {
				inner(child, f)?;
			}
			Ok(())
		}
		writeln!(f, "Routes:")?;
		inner(self, f)
	}
}
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn action_at(path: &str) -> impl Bundle {
		(
			PathPartial::new(path),
			Action::<(), ()>::new_pure(|_: ActionContext| Ok(())),
		)
	}

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	#[beet_core::test]
	fn builds_tree_on_spawn() {
		let mut world = router_world();
		let root = world
			.spawn(children![action_at("foo"), action_at("bar")])
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.flatten().len().xpect_eq(2);
		tree.flatten_action_nodes().len().xpect_eq(2);
	}

	#[beet_core::test]
	fn nested_paths() {
		let mut world = router_world();
		let root = world
			.spawn((PathPartial::new("api"), children![
				action_at("users"),
				action_at("posts")
			]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		let paths: Vec<String> = tree
			.flatten()
			.iter()
			.map(|p| p.annotated_path().to_string())
			.collect();
		paths.contains(&"api/users".to_string()).xpect_true();
		paths.contains(&"api/posts".to_string()).xpect_true();
	}

	/// A server mounted under a command route serves the `Router` *beneath* it,
	/// not the command url space it is addressed from; a route inside a url
	/// space still resolves to the space holding it.
	///
	/// Regression: `beet --main=site serve --server=tui` resolved the command
	/// dispatcher's tree, so its home page opened on "no route matched /".
	#[beet_core::test]
	fn resolves_a_mounted_servers_own_router() {
		let mut world = router_world();
		let commands = world.spawn(Router).id();
		let server = world
			.spawn((ChildOf(commands), PathPartial::new("serve")))
			.id();
		let site = world.spawn((ChildOf(server), Router)).id();
		let page = world
			.spawn((ChildOf(site), PageRoute, action_at("about")))
			.id();
		world.flush();

		let resolve = |entity: Entity, world: &mut World| {
			world
				.run_system_cached_with::<_, Result<Entity>, _, _>(
					find_router,
					entity,
				)
				.unwrap()
				.unwrap()
		};
		// the mounted server dispatches into its own url space..
		resolve(server, &mut world).xpect_eq(site);
		// ..which is also the site an entry root means, over its commands
		resolve(commands, &mut world).xpect_eq(site);
		// ..and a route resolves the space it belongs to
		resolve(page, &mut world).xpect_eq(site);
	}

	#[beet_core::test]
	fn find_by_path() {
		let mut world = router_world();
		let root = world
			.spawn(children![action_at("foo"), action_at("bar")])
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.find(&["foo"]).xpect_some();
		tree.find(&["bar"]).xpect_some();
		tree.find(&["baz"]).xpect_none();
	}

	#[beet_core::test]
	fn find_nested_path() {
		let mut world = router_world();
		let root = world
			.spawn(children![(PathPartial::new("counter"), children![
				action_at("increment"),
				action_at("decrement")
			])])
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.find(&["counter", "increment"]).xpect_some();
		tree.find(&["counter", "decrement"]).xpect_some();
		tree.find(&["counter"]).xpect_none();
	}

	#[beet_core::test]
	fn scene_routes_appear_in_route_tree() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"about"</p> }
				),
				action_at("action"),
			])
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.flatten_scene_nodes().len().xpect_eq(1);
		tree.flatten_action_nodes().len().xpect_eq(1);
	}

	#[beet_core::test]
	fn detects_duplicate_paths() {
		let nodes = vec![
			ActionNode {
				entity: Entity::PLACEHOLDER,
				meta: ActionMeta::of::<(), (), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("foo").unwrap(),
				method: None,
				is_page_route: false,
			},
			ActionNode {
				entity: Entity::PLACEHOLDER,
				meta: ActionMeta::of::<(), (), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("foo").unwrap(),
				method: None,
				is_page_route: false,
			},
		];
		RouteTree::from_nodes(nodes)
			.unwrap_err()
			.to_string()
			.contains("Duplicate route")
			.xpect_true();
	}

	#[beet_core::test]
	fn detects_dynamic_conflicts() {
		let nodes = vec![
			ActionNode {
				entity: Entity::PLACEHOLDER,
				meta: ActionMeta::of::<(), (), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new(":foo").unwrap(),
				method: None,
				is_page_route: false,
			},
			ActionNode {
				entity: Entity::PLACEHOLDER,
				meta: ActionMeta::of::<(), (), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new(":bar").unwrap(),
				method: None,
				is_page_route: false,
			},
		];
		RouteTree::from_nodes(nodes)
			.unwrap_err()
			.to_string()
			.contains("Path conflict")
			.xpect_true();
	}

	#[beet_core::test]
	fn detects_static_dynamic_mix() {
		let nodes = vec![
			ActionNode {
				entity: Entity::PLACEHOLDER,
				meta: ActionMeta::of::<(), (), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("foo").unwrap(),
				method: None,
				is_page_route: false,
			},
			ActionNode {
				entity: Entity::PLACEHOLDER,
				meta: ActionMeta::of::<(), (), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new(":bar").unwrap(),
				method: None,
				is_page_route: false,
			},
		];
		RouteTree::from_nodes(nodes)
			.unwrap_err()
			.to_string()
			.contains("Path conflict")
			.xpect_true();
	}

	#[beet_core::test]
	fn allows_different_static_paths() {
		let nodes = vec![
			ActionNode {
				entity: Entity::PLACEHOLDER,
				meta: ActionMeta::of::<(), (), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("foo").unwrap(),
				method: None,
				is_page_route: false,
			},
			ActionNode {
				entity: Entity::PLACEHOLDER,
				meta: ActionMeta::of::<(), (), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("bar").unwrap(),
				method: None,
				is_page_route: false,
			},
		];
		let tree = RouteTree::from_nodes(nodes).unwrap();
		tree.flatten().len().xpect_eq(2);
	}

	#[beet_core::test]
	fn display_format() {
		let mut world = router_world();
		let root = world
			.spawn(children![action_at("foo"), action_at("bar"),])
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		let output = tree.to_string();
		output.contains("Routes:").xpect_true();
		output.contains("bar").xpect_true();
		output.contains("foo").xpect_true();
	}

	#[beet_core::test]
	fn flatten_nodes_returns_all_routes() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				action_at("alpha"),
				action_at("beta"),
				(PathPartial::new("nested"), children![action_at("gamma")])
			])
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		// 3 actions
		tree.flatten_nodes().len().xpect_eq(3);
		tree.flatten_action_nodes().len().xpect_eq(3);
	}

	/// A tree with a nested folder, used to pin the iterator orderings: a root
	/// over `outer` and an `inner` folder holding `deep`.
	fn nested_tree(world: &mut World) -> Entity {
		world
			.spawn(children![
				action_at("outer"),
				(PathPartial::new("inner"), children![action_at("deep")])
			])
			.flush()
	}

	/// Maps a node list to its annotated paths for order assertions.
	fn paths_of(nodes: &[&RouteTree]) -> Vec<String> {
		nodes
			.iter()
			.map(|tree| tree.path.annotated_path().to_string())
			.collect()
	}

	/// DFS yields each node before its children: the `inner` folder node comes
	/// immediately before its `deep` child, ahead of the sorted-later `outer`.
	#[beet_core::test]
	fn iter_dfs_is_reading_order() {
		let mut world = router_world();
		let root = nested_tree(&mut world);
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		// root (empty), then sorted children: inner before its deep child, then outer
		paths_of(&tree.iter_dfs()).xpect_eq(vec![
			"".to_string(),
			"inner".to_string(),
			"inner/deep".to_string(),
			"outer".to_string(),
		]);
	}

	/// BFS yields every node at a depth before any deeper node: both top-level
	/// nodes precede the nested `deep` child.
	#[beet_core::test]
	fn iter_bfs_is_level_order() {
		let mut world = router_world();
		let root = nested_tree(&mut world);
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		// root, then the full first level, then the deeper child
		paths_of(&tree.iter_bfs()).xpect_eq(vec![
			"".to_string(),
			"inner".to_string(),
			"outer".to_string(),
			"inner/deep".to_string(),
		]);
	}

	#[beet_core::test]
	fn tracks_action_entities() {
		let mut world = router_world();
		let root = world.spawn(children![action_at("tracked")]).flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		let node = tree.find(&["tracked"]).unwrap();
		// the entity should be valid and queryable
		world
			.entity(node.entity)
			.contains::<ActionMeta>()
			.xpect_true();
	}

	#[beet_core::test]
	fn find_subtree_returns_scoped_nodes() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				(
					render_action::fixed_func_route("counter", || {
						Element::new("p").with_inner_text("counter")
					}),
					children![action_at("increment"), action_at("decrement"),],
				),
				action_at("other"),
			])
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		let subtree = tree.find_subtree(&["counter"]).unwrap();
		// subtree contains the counter scene route + 2 actions
		subtree.flatten_nodes().len().xpect_eq(3);
		subtree.flatten_action_nodes().len().xpect_eq(2);
		// sibling action should not appear in subtree
		subtree
			.flatten_nodes()
			.iter()
			.any(|node| {
				node.path.annotated_path().to_string().contains("other")
			})
			.xpect_false();
	}

	#[beet_core::test]
	fn find_subtree_returns_none_for_missing_prefix() {
		let mut world = router_world();
		let root = world.spawn(children![action_at("foo")]).flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.find_subtree(&["nonexistent"]).xpect_none();
	}

	#[beet_core::test]
	fn find_subtree_falls_back_to_dynamic_segment() {
		let mut world = router_world();
		let root = world
			.spawn(children![(PathPartial::new(":id"), children![action_at(
				"details"
			),])])
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		// no static "42" child exists, should fall back to :id
		let subtree = tree.find_subtree(&["42"]).unwrap();
		subtree.flatten_action_nodes().len().xpect_eq(1);
		subtree
			.flatten_action_nodes()
			.first()
			.unwrap()
			.path
			.annotated_path()
			.to_string()
			.xpect_contains("details");
	}
}
