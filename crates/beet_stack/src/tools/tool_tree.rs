use crate::prelude::*;
use beet_core::prelude::*;

/// Collects all tools in an entity hierarchy and arranges them into a validated tree.
///
/// Inserted on the root ancestor whenever a tool's [`PathPattern`] is set.
/// Ensures there is only a single tool for any given path pattern and
/// detects conflicts between dynamic and greedy segments that would
/// cause ambiguous routing.
///
/// ## Validation Rules
/// - Only one tool per exact path pattern
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
/// let tree = world.entity(root).get::<ToolTree>().unwrap();
/// tree.flatten().len().xpect_eq(2);
/// ```
#[derive(Debug, Clone, Component)]
pub struct ToolTree {
	/// The path pattern for this tree node.
	pub pattern: PathPattern,
	/// The params pattern for this tree node.
	pub params: ParamsPattern,
	/// The tool at this exact path, if any.
	tool: Option<ToolNode>,
	/// Child nodes in the tree.
	pub children: Vec<ToolTree>,
}

impl ToolTree {
	/// Returns the [`ToolNode`] at this level of the tree, if any.
	pub fn node(&self) -> Option<&ToolNode> { self.tool.as_ref() }

	/// Builds a [`ToolTree`] from a list of [`ToolNode`].
	///
	/// ## Errors
	///
	/// Returns an error if there are conflicting or duplicate paths.
	pub fn from_nodes(nodes: Vec<ToolNode>) -> Result<Self> {
		#[derive(Default)]
		struct Node {
			children: HashMap<String, Node>,
			tool: Option<ToolNode>,
			params: Option<ParamsPattern>,
			is_static: Option<bool>,
		}

		let mut root = Node::default();

		for tool_node in &nodes {
			let segments = tool_node.path.iter().cloned().collect::<Vec<_>>();
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
					tool: None,
					params: None,
					children: default(),
				});

				if is_last {
					if node.tool.is_some() {
						bevybail!(
							"Duplicate tool: multiple tools defined for path '{}'",
							tool_node.path.annotated_route_path()
						);
					}
					node.tool = Some(tool_node.clone());
					node.params = Some(tool_node.params.clone());
				}
			}

			// handle root path (empty segments)
			if segments.is_empty() {
				if node.tool.is_some() {
					bevybail!(
						"Duplicate tool: multiple tools defined for path '/'"
					);
				}
				node.tool = Some(tool_node.clone());
				node.params = Some(tool_node.params.clone());
			}
		}

		/// Recursively build the ToolTree, sorting children by their path.
		fn build_tree(
			pattern: PathPattern,
			params: ParamsPattern,
			node: &Node,
		) -> ToolTree {
			let mut children: Vec<ToolTree> = node
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

			children.sort_by(|a, b| a.pattern.cmp(&b.pattern));

			ToolTree {
				pattern,
				params: node.params.clone().unwrap_or(params),
				tool: node.tool.clone(),
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

	/// Returns all tool paths in the tree as a flat list.
	pub fn flatten(&self) -> Vec<PathPattern> {
		let mut patterns = Vec::new();
		fn inner(patterns: &mut Vec<PathPattern>, node: &ToolTree) {
			if node.tool.is_some() {
				patterns.push(node.pattern.clone());
			}
			for child in &node.children {
				inner(patterns, child);
			}
		}
		inner(&mut patterns, self);
		patterns
	}

	/// Returns all tool nodes in the tree as a flat list.
	pub fn flatten_nodes(&self) -> Vec<&ToolNode> {
		let mut nodes = Vec::new();
		fn inner<'a>(nodes: &mut Vec<&'a ToolNode>, tree: &'a ToolTree) {
			if let Some(tool) = &tree.tool {
				nodes.push(tool);
			}
			for child in &tree.children {
				inner(nodes, child);
			}
		}
		inner(&mut nodes, self);
		nodes
	}

	/// Find a tool node matching the given path segments.
	///
	/// Walks the tree looking for an exact match against
	/// the provided path.
	pub fn find(&self, path: &[impl AsRef<str>]) -> Option<&ToolNode> {
		let path_vec: Vec<String> =
			path.iter().map(|s| s.as_ref().to_string()).collect();

		fn inner<'a>(
			node: &'a ToolTree,
			path: &Vec<String>,
		) -> Option<&'a ToolNode> {
			if let Some(tool) = &node.tool {
				if tool
					.path
					.parse_path(path)
					.map(|m| m.exact_match())
					.unwrap_or(false)
				{
					return Some(tool);
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
}

impl std::fmt::Display for ToolTree {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		fn inner(
			node: &ToolTree,
			f: &mut std::fmt::Formatter<'_>,
		) -> std::fmt::Result {
			if let Some(tool) = &node.tool {
				let path = node.pattern.annotated_route_path();
				let input = tool.meta.input().type_name();
				let output = tool.meta.output().type_name();
				write!(f, "  {}", path)?;
				if let Some(method) = &tool.method {
					write!(f, " [{}]", method)?;
				}
				writeln!(f)?;
				writeln!(f, "    input:  {}", input)?;
				writeln!(f, "    output: {}", output)?;
				for param in node.params.iter() {
					writeln!(f, "    {}", param)?;
				}
			}
			for child in &node.children {
				inner(child, f)?;
			}
			Ok(())
		}
		writeln!(f, "Tools:")?;
		inner(self, f)
	}
}

/// Observer that rebuilds the [`ToolTree`] on the root ancestor
/// whenever a [`PathPattern`] is inserted on any entity in the hierarchy.
///
/// Collects all entities with tool components ([`ToolMeta`], [`PathPattern`],
/// [`ParamsPattern`]) from the root's descendants and constructs a validated tree.
pub fn insert_tool_tree(
	ev: On<Insert, PathPattern>,
	ancestors: Query<&ChildOf>,
	children_query: Query<&Children>,
	tools: Query<(
		Entity,
		&ToolMeta,
		&PathPattern,
		&ParamsPattern,
		Option<&HttpMethod>,
	)>,
	mut commands: Commands,
) -> Result {
	let root = ancestors.root_ancestor(ev.entity);
	let nodes: Vec<ToolNode> = children_query
		.iter_descendants_inclusive(root)
		.filter_map(|entity| {
			tools.get(entity).ok().map(
				|(entity, meta, path, params, method)| {
					ToolNode::from_query(entity, meta, path, params, method)
				},
			)
		})
		.collect();

	if nodes.is_empty() {
		return Ok(());
	}

	let tree = ToolTree::from_nodes(nodes)?;
	commands.entity(root).insert(tree);
	Ok(())
}


/// A single tool node in the tree, representing a registered tool at a specific path.
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
}

impl ToolNode {
	/// Create a [`ToolNode`] from query results.
	pub fn from_query(
		entity: Entity,
		meta: &ToolMeta,
		path: &PathPattern,
		params: &ParamsPattern,
		method: Option<&HttpMethod>,
	) -> Self {
		Self {
			entity,
			meta: meta.clone(),
			params: params.clone(),
			path: path.clone(),
			method: method.cloned(),
		}
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
		let tree = world.entity(root).get::<ToolTree>().unwrap();
		tree.flatten().len().xpect_eq(2);
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
		let tree = world.entity(root).get::<ToolTree>().unwrap();
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
		let tree = world.entity(root).get::<ToolTree>().unwrap();
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
		let tree = world.entity(root).get::<ToolTree>().unwrap();
		tree.find(&["counter", "increment"]).xpect_some();
		tree.find(&["counter", "decrement"]).xpect_some();
		tree.find(&["counter"]).xpect_none();
	}

	#[test]
	fn detects_duplicate_paths() {
		let nodes = vec![
			ToolNode {
				entity: Entity::PLACEHOLDER,
				meta: ToolMeta::of::<(), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("foo").unwrap(),
				method: None,
			},
			ToolNode {
				entity: Entity::PLACEHOLDER,
				meta: ToolMeta::of::<(), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("foo").unwrap(),
				method: None,
			},
		];
		ToolTree::from_nodes(nodes)
			.unwrap_err()
			.to_string()
			.contains("Duplicate tool")
			.xpect_true();
	}

	#[test]
	fn detects_dynamic_conflicts() {
		let nodes = vec![
			ToolNode {
				entity: Entity::PLACEHOLDER,
				meta: ToolMeta::of::<(), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new(":foo").unwrap(),
				method: None,
			},
			ToolNode {
				entity: Entity::PLACEHOLDER,
				meta: ToolMeta::of::<(), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new(":bar").unwrap(),
				method: None,
			},
		];
		ToolTree::from_nodes(nodes)
			.unwrap_err()
			.to_string()
			.contains("Path conflict")
			.xpect_true();
	}

	#[test]
	fn allows_different_static_paths() {
		let nodes = vec![
			ToolNode {
				entity: Entity::PLACEHOLDER,
				meta: ToolMeta::of::<(), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("foo").unwrap(),
				method: None,
			},
			ToolNode {
				entity: Entity::PLACEHOLDER,
				meta: ToolMeta::of::<(), ()>(),
				params: ParamsPattern::default(),
				path: PathPattern::new("bar").unwrap(),
				method: None,
			},
		];
		let tree = ToolTree::from_nodes(nodes).unwrap();
		tree.flatten().len().xpect_eq(2);
	}

	#[test]
	fn display_format() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, children![tool_at("foo"), tool_at("bar"),]))
			.flush();
		let tree = world.entity(root).get::<ToolTree>().unwrap();
		let output = tree.to_string();
		output.contains("Tools:").xpect_true();
		output.contains("bar").xpect_true();
		output.contains("foo").xpect_true();
	}

	#[test]
	fn flatten_nodes_returns_all_tools() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((Card, children![
				tool_at("alpha"),
				tool_at("beta"),
				(PathPartial::new("nested"), children![tool_at("gamma")])
			]))
			.flush();
		let tree = world.entity(root).get::<ToolTree>().unwrap();
		tree.flatten_nodes().len().xpect_eq(3);
	}

	#[test]
	fn tracks_tool_entities() {
		let mut world = StackPlugin::world();
		let root = world.spawn((Card, children![tool_at("tracked")])).flush();
		let tree = world.entity(root).get::<ToolTree>().unwrap();
		let node = tree.find(&["tracked"]).unwrap();
		// the entity should be valid and queryable
		world
			.entity(node.entity)
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
		let tree = world.entity(root).get::<ToolTree>().unwrap();
		tree.find(&["increment"]).xpect_some();
		tree.find(&["decrement"]).xpect_some();
		tree.flatten().len().xpect_eq(2);
	}
}
