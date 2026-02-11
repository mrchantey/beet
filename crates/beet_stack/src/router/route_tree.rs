use crate::prelude::*;
use beet_core::prelude::*;

/// Marker component that excludes an entity from the [`RouteTree`].
///
/// Internal tools like fallback chain handlers should not appear
/// as routable endpoints. Adding this component prevents them from
/// being collected during route tree construction.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct RouteHidden;

/// Collects all routes (tools and cards) in an entity hierarchy and
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
///
/// ## Example
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = StackPlugin::world();
/// let root = world.spawn((Card, children![
///     increment(FieldRef::new("count")),
///     decrement(FieldRef::new("count")),
/// ])).flush();
///
/// let tree = world.entity(root).get::<RouteTree>().unwrap();
/// tree.flatten_tool_nodes().len().xpect_eq(2);
/// ```
#[derive(Debug, Clone, Component)]
pub struct RouteTree {
	/// The path pattern for this tree node.
	pub path: PathPattern,
	/// The params pattern for this tree node.
	pub params: ParamsPattern,
	/// The route at this exact path, if any.
	node: Option<RouteNode>,
	/// Child nodes in the tree.
	pub children: Vec<RouteTree>,
}

/// A single route node in the tree, representing either a card or tool
/// at a specific path.
#[derive(Debug, Clone)]
pub enum RouteNode {
	/// A card route, representing navigable content.
	Card(CardNode),
	/// A tool route, representing a callable action.
	Tool(ToolNode),
}

impl RouteNode {
	/// Returns the entity for this route node.
	pub fn entity(&self) -> Entity {
		match self {
			RouteNode::Card(card) => card.entity,
			RouteNode::Tool(tool) => tool.entity,
		}
	}

	/// Returns the path pattern for this route node.
	pub fn path(&self) -> &PathPattern {
		match self {
			RouteNode::Card(card) => &card.path,
			RouteNode::Tool(tool) => &tool.path,
		}
	}

	/// Returns the params pattern for this route node.
	pub fn params(&self) -> &ParamsPattern {
		match self {
			RouteNode::Card(card) => &card.params,
			RouteNode::Tool(tool) => &tool.params,
		}
	}

	/// Returns true if this route is a card.
	pub fn is_card(&self) -> bool { matches!(self, RouteNode::Card(_)) }

	/// Returns true if this route is a tool.
	pub fn is_tool(&self) -> bool { matches!(self, RouteNode::Tool(_)) }

	/// Returns the inner [`ToolNode`] if this is a tool route.
	pub fn as_tool(&self) -> Option<&ToolNode> {
		match self {
			RouteNode::Tool(tool) => Some(tool),
			_ => None,
		}
	}

	/// Returns the inner [`CardNode`] if this is a card route.
	pub fn as_card(&self) -> Option<&CardNode> {
		match self {
			RouteNode::Card(card) => Some(card),
			_ => None,
		}
	}
}


/// A card route node, representing navigable content at a specific path.
#[derive(Debug, Clone)]
pub struct CardNode {
	/// The entity containing this card.
	pub entity: Entity,
	/// The parameter pattern for this card.
	pub params: ParamsPattern,
	/// The full path pattern for this card.
	pub path: PathPattern,
}

/// A tool route node, representing a callable action at a specific path.
#[derive(Debug, Clone)]
pub struct ToolNode {
	/// The entity containing this tool.
	pub entity: Entity,
	/// Metadata about the tool's input/output types.
	pub meta: ToolMeta,
	/// The parameter pattern for this tool.
	pub params: ParamsPattern,
	/// The full path pattern for this tool.
	pub path: PathPattern,
	/// Optional HTTP method restriction.
	pub method: Option<HttpMethod>,
	/// Whether this tool also accepts opaque [`Request`]/[`Response`] calls,
	/// ie created via [`exchange_tool`] or via `tool()` with the `interface` feature.
	pub is_exchange: bool,
}

/// The query tuple type used to collect tool components for [`ToolNode::from_query`].
pub type ToolQueryItem<'a> = (
	Entity,
	&'a ToolMeta,
	&'a PathPattern,
	&'a ParamsPattern,
	Option<&'a HttpMethod>,
	bool,
);

impl ToolNode {
	/// Create a [`ToolNode`] from the full query result tuple.
	pub fn from_query(
		(entity, meta, path, params, method, is_exchange): ToolQueryItem,
	) -> Self {
		Self {
			entity,
			meta: meta.clone(),
			params: params.clone(),
			path: path.clone(),
			method: method.cloned(),
			is_exchange,
		}
	}
}

/// The query tuple type used to collect card components for [`CardNode::from_query`].
pub type CardQueryItem<'a> = (Entity, &'a PathPattern, &'a ParamsPattern);

impl CardNode {
	/// Create a [`CardNode`] from the query result tuple.
	pub fn from_query((entity, path, params): CardQueryItem) -> Self {
		Self {
			entity,
			params: params.clone(),
			path: path.clone(),
		}
	}
}


impl RouteTree {
	/// Returns the [`RouteNode`] at this level of the tree, if any.
	pub fn node(&self) -> Option<&RouteNode> { self.node.as_ref() }

	/// Builds a [`RouteTree`] from a list of [`RouteNode`].
	///
	/// ## Errors
	///
	/// Returns an error if there are conflicting or duplicate paths.
	pub fn from_nodes(nodes: Vec<RouteNode>) -> Result<Self> {
		#[derive(Default)]
		struct Node {
			children: HashMap<String, Node>,
			route: Option<RouteNode>,
			params: Option<ParamsPattern>,
			is_static: Option<bool>,
		}

		let mut root = Node::default();

		for route_node in &nodes {
			let path = route_node.path();
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
							path.annotated_route_path()
						);
					}
					node.route = Some(route_node.clone());
					node.params = Some(route_node.params().clone());
				}
			}

			// handle root path (empty segments)
			if segments.is_empty() {
				if node.route.is_some() {
					bevybail!(
						"Duplicate route: multiple routes defined for path '/'"
					);
				}
				node.route = Some(route_node.clone());
				node.params = Some(route_node.params().clone());
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

	/// Returns all route paths in the tree as a flat list.
	/// Nodes with no matching route are skipped.
	pub fn flatten(&self) -> Vec<PathPattern> {
		let mut patterns = Vec::new();
		fn inner(patterns: &mut Vec<PathPattern>, node: &RouteTree) {
			if node.node.is_some() {
				patterns.push(node.path.clone());
			}
			for child in &node.children {
				inner(patterns, child);
			}
		}
		inner(&mut patterns, self);
		patterns
	}

	/// Returns all route nodes in the tree as a flat list.
	pub fn flatten_nodes(&self) -> Vec<&RouteNode> {
		let mut nodes = Vec::new();
		fn inner<'a>(nodes: &mut Vec<&'a RouteNode>, tree: &'a RouteTree) {
			if let Some(route) = &tree.node {
				nodes.push(route);
			}
			for child in &tree.children {
				inner(nodes, child);
			}
		}
		inner(&mut nodes, self);
		nodes
	}

	/// Returns all tool nodes in the tree as a flat list, skipping card nodes.
	pub fn flatten_tool_nodes(&self) -> Vec<&ToolNode> {
		self.flatten_nodes()
			.into_iter()
			.filter_map(|node| node.as_tool())
			.collect()
	}

	/// Returns all card nodes in the tree as a flat list, skipping tool nodes.
	pub fn flatten_card_nodes(&self) -> Vec<&CardNode> {
		self.flatten_nodes()
			.into_iter()
			.filter_map(|node| node.as_card())
			.collect()
	}

	/// Find a route node matching the given path segments.
	///
	/// Walks the tree looking for an exact match against
	/// the provided path. There should never be more than one match
	/// as [`RouteTree::from_nodes`] rejects conflicts.
	pub fn find(&self, path: &[impl AsRef<str>]) -> Option<&RouteNode> {
		let path_vec: Vec<String> =
			path.iter().map(|s| s.as_ref().to_string()).collect();

		fn inner<'a>(
			node: &'a RouteTree,
			path: &Vec<String>,
		) -> Option<&'a RouteNode> {
			if let Some(route) = &node.node {
				if route
					.path()
					.parse_path(path)
					.map(|m| m.exact_match())
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

	/// Find a tool node matching the given path segments.
	///
	/// Convenience method that calls [`find`](Self::find) and filters
	/// for tool routes only.
	pub fn find_tool(&self, path: &[impl AsRef<str>]) -> Option<&ToolNode> {
		self.find(path).and_then(|node| node.as_tool())
	}

	/// Find a card node matching the given path segments.
	///
	/// Convenience method that calls [`find`](Self::find) and filters
	/// for card routes only.
	pub fn find_card(&self, path: &[impl AsRef<str>]) -> Option<&CardNode> {
		self.find(path).and_then(|node| node.as_card())
	}

	/// Find the subtree rooted at the given path prefix.
	///
	/// Walks the tree children matching each segment of `prefix` in
	/// turn, returning the [`RouteTree`] node at that position. This
	/// is useful for scoping help output to a specific path prefix.
	///
	/// Returns `None` if no tree node matches the prefix.
	pub fn find_subtree(
		&self,
		prefix: &[impl AsRef<str>],
	) -> Option<&RouteTree> {
		let mut current = self;
		for segment in prefix {
			let seg = segment.as_ref();
			current = current.children.iter().find(|child| {
				child
					.path
					.iter()
					.last()
					.map(|last| last.is_static() && last.name() == seg)
					.unwrap_or(false)
			})?;
		}
		Some(current)
	}
}

impl std::fmt::Display for RouteTree {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		fn inner(
			node: &RouteTree,
			f: &mut std::fmt::Formatter<'_>,
		) -> std::fmt::Result {
			if let Some(route) = &node.node {
				let path = node.path.annotated_route_path();
				match route {
					RouteNode::Card(_) => {
						writeln!(f, "  {} [card]", path)?;
					}
					RouteNode::Tool(tool) => {
						let input = tool.meta.input().type_name();
						let output = tool.meta.output().type_name();
						write!(f, "  {}", path)?;
						if let Some(method) = &tool.method {
							write!(f, " [{}]", method)?;
						}
						writeln!(f)?;
						writeln!(f, "    input:  {}", input)?;
						writeln!(f, "    output: {}", output)?;
					}
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
	use beet_core::prelude::*;

	fn tool_at(path: &str) -> impl Bundle {
		(PathPartial::new(path), tool(|| {}))
	}

	#[test]
	fn builds_tree_on_spawn() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, children![tool_at("foo"), tool_at("bar")]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		// 2 tools + 1 root card = 3 routes
		tree.flatten().len().xpect_eq(3);
		tree.flatten_tool_nodes().len().xpect_eq(2);
	}

	#[test]
	fn nested_paths() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, PathPartial::new("api"), children![
				tool_at("users"),
				tool_at("posts")
			]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		let paths: Vec<String> = tree
			.flatten()
			.iter()
			.map(|p| p.annotated_route_path().to_string())
			.collect();
		paths.contains(&"/api/users".to_string()).xpect_true();
		paths.contains(&"/api/posts".to_string()).xpect_true();
	}

	#[test]
	fn find_by_path() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, children![tool_at("foo"), tool_at("bar")]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.find(&["foo"]).xpect_some();
		tree.find(&["bar"]).xpect_some();
		tree.find(&["baz"]).xpect_none();
	}

	#[test]
	fn find_nested_path() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, children![(PathPartial::new("counter"), children![
				tool_at("increment"),
				tool_at("decrement")
			])]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.find(&["counter", "increment"]).xpect_some();
		tree.find(&["counter", "decrement"]).xpect_some();
		tree.find(&["counter"]).xpect_none();
	}

	#[test]
	fn find_tool_filters_correctly() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, children![tool_at("my-tool"), card("my-card"),]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.find_tool(&["my-tool"]).xpect_some();
		tree.find_tool(&["my-card"]).xpect_none();
		tree.find_card(&["my-card"]).xpect_some();
		tree.find_card(&["my-tool"]).xpect_none();
	}

	#[test]
	fn cards_appear_in_route_tree() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, children![card("about"), tool_at("action"),]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		// 1 child card + 1 root card + 1 tool = 3 routes
		tree.flatten().len().xpect_eq(3);
		// root card + about card
		tree.flatten_card_nodes().len().xpect_eq(2);
		tree.flatten_tool_nodes().len().xpect_eq(1);
	}

	#[test]
	fn detects_duplicate_paths() {
		let nodes = vec![
			RouteNode::Tool(ToolNode {
				entity: Entity::PLACEHOLDER,
				meta: ToolMeta::of::<(), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("foo").unwrap(),
				method: None,
				is_exchange: false,
			}),
			RouteNode::Tool(ToolNode {
				entity: Entity::PLACEHOLDER,
				meta: ToolMeta::of::<(), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("foo").unwrap(),
				method: None,
				is_exchange: false,
			}),
		];
		RouteTree::from_nodes(nodes)
			.unwrap_err()
			.to_string()
			.contains("Duplicate route")
			.xpect_true();
	}

	#[test]
	fn detects_dynamic_conflicts() {
		let nodes = vec![
			RouteNode::Tool(ToolNode {
				entity: Entity::PLACEHOLDER,
				meta: ToolMeta::of::<(), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new(":foo").unwrap(),
				method: None,
				is_exchange: false,
			}),
			RouteNode::Tool(ToolNode {
				entity: Entity::PLACEHOLDER,
				meta: ToolMeta::of::<(), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new(":bar").unwrap(),
				method: None,
				is_exchange: false,
			}),
		];
		RouteTree::from_nodes(nodes)
			.unwrap_err()
			.to_string()
			.contains("Path conflict")
			.xpect_true();
	}

	#[test]
	fn allows_different_static_paths() {
		let nodes = vec![
			RouteNode::Tool(ToolNode {
				entity: Entity::PLACEHOLDER,
				meta: ToolMeta::of::<(), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("foo").unwrap(),
				method: None,
				is_exchange: false,
			}),
			RouteNode::Tool(ToolNode {
				entity: Entity::PLACEHOLDER,
				meta: ToolMeta::of::<(), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("bar").unwrap(),
				method: None,
				is_exchange: false,
			}),
		];
		let tree = RouteTree::from_nodes(nodes).unwrap();
		tree.flatten().len().xpect_eq(2);
	}

	#[test]
	fn display_format() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, children![tool_at("foo"), tool_at("bar"),]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		let output = tree.to_string();
		output.contains("Routes:").xpect_true();
		output.contains("bar").xpect_true();
		output.contains("foo").xpect_true();
	}

	#[test]
	fn flatten_nodes_returns_all_routes() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, children![
				tool_at("alpha"),
				tool_at("beta"),
				(PathPartial::new("nested"), children![tool_at("gamma")])
			]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		// 3 tools + 1 root card = 4 routes
		tree.flatten_nodes().len().xpect_eq(4);
		tree.flatten_tool_nodes().len().xpect_eq(3);
	}

	#[test]
	fn tracks_tool_entities() {
		let mut world = StackPlugin::world();
		let root = world.spawn((Card, children![tool_at("tracked")])).flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		let node = tree.find(&["tracked"]).unwrap();
		// the entity should be valid and queryable
		world
			.entity(node.entity())
			.contains::<ToolMeta>()
			.xpect_true();
	}

	#[test]
	fn common_tools_tree() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, children![
				increment(FieldRef::new("count")),
				decrement(FieldRef::new("count")),
			]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.find(&["increment"]).xpect_some();
		tree.find(&["decrement"]).xpect_some();
		// 2 tools + 1 root card = 3 routes
		tree.flatten().len().xpect_eq(3);
		tree.flatten_tool_nodes().len().xpect_eq(2);
	}

	#[test]
	fn find_subtree_returns_scoped_nodes() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, children![
				(card("counter"), children![
					tool_at("increment"),
					tool_at("decrement"),
				]),
				tool_at("other"),
			]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		let subtree = tree.find_subtree(&["counter"]).unwrap();
		// subtree contains the counter card + 2 tools
		subtree.flatten_nodes().len().xpect_eq(3);
		subtree.flatten_tool_nodes().len().xpect_eq(2);
		// sibling tool should not appear in subtree
		subtree
			.flatten_nodes()
			.iter()
			.any(|node| {
				node.path()
					.annotated_route_path()
					.to_string()
					.contains("other")
			})
			.xpect_false();
	}

	#[test]
	fn find_subtree_returns_none_for_missing_prefix() {
		let mut world = StackPlugin::world();
		let root = world.spawn((Card, children![tool_at("foo")])).flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.find_subtree(&["nonexistent"]).xpect_none();
	}
}
