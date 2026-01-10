use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Spawns all ExchangeSpawner trees and collects Endpoint components.
/// Returns the list of endpoints and the entities that were spawned (which the caller must despawn).
pub(crate) fn spawn_and_collect_endpoints(
	world: &mut World,
) -> (Vec<(Entity, PathPattern, ParamsPattern)>, Vec<Entity>) {
	let spawners: Vec<ExchangeSpawner> = world
		.query_once::<&ExchangeSpawner>()
		.iter()
		.map(|s| (*s).clone())
		.collect();

	let mut spawned_roots = Vec::new();
	for spawner in &spawners {
		spawned_roots.push(spawner.spawn(world));
	}

	let endpoints: Vec<(Entity, PathPattern, ParamsPattern)> = world
		.query_once::<(Entity, &Endpoint)>()
		.iter()
		.map(|(entity, endpoint)| {
			(*entity, endpoint.path().clone(), endpoint.params().clone())
		})
		.collect();

	(endpoints, spawned_roots)
}

/// Collects all endpoints in an application and arranges them into a tree structure.
///
/// This serves as a validation step, ensuring there is only a single endpoint for
/// any given path pattern. It also detects conflicts between dynamic and greedy
/// segments that would cause ambiguous routing.
///
/// ## Validation Rules
/// - Only one endpoint per exact path pattern
/// - Cannot mix static and dynamic segments at the same level (e.g., `/api/users` and `/api/:id`)
/// - Cannot have multiple dynamic segments at the same level (e.g., `/:foo` and `/:bar`)
/// - Cannot have multiple greedy segments at the same level (e.g., `*foo` and `*bar`)
/// - Greedy segments (OneOrMore, ZeroOrMore) must be the last segment in a path
///
/// ## Example
/// ```rust
/// # use beet_router::prelude::*;
/// # use beet_flow::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
///
/// let mut world = World::new();
/// world.spawn(ExchangeSpawner::new_flow(|| {
///   (
/// 		InfallibleSequence, children![
///       EndpointBuilder::get().with_path("api"),
///       EndpointBuilder::get().with_path("users/:id"),
///       EndpointBuilder::get().with_path("docs/*path"),
///     ]
/// 	)
/// }));
///
/// // build the tree and validate all paths
/// let tree = EndpointTree::from_world(&mut world).unwrap();
/// ```
// TODO this should be a component, inserted alongside the ExchangeSpawner
#[derive(Debug, Clone, Resource)]
pub struct EndpointTree {
	/// The path pattern for this node
	pub pattern: PathPattern,
	/// The params pattern for this node
	pub params: ParamsPattern,
	/// The entity with an [`Endpoint`] at this exact path, if any
	pub endpoint: Option<Entity>,
	/// Child nodes in the tree
	pub children: Vec<EndpointTree>,
}

impl EndpointTree {
	/// Builds an [`EndpointTree`] from all endpoints in the world.
	///
	/// This spawns trees from all [`ExchangeSpawner`]s to collect their endpoints,
	/// then despawns those temporary trees after collection.
	///
	/// Returns an error if there are conflicting paths.
	pub fn from_world(world: &mut World) -> Result<Self> {
		let (endpoints, spawned_roots) = spawn_and_collect_endpoints(world);

		// Build the tree before cleanup (endpoints reference these entities)
		let tree = Self::from_endpoints(endpoints);

		// Despawn the temporary trees
		for root in spawned_roots {
			world.entity_mut(root).despawn();
		}

		tree
	}

	/// Builds an [`EndpointTree`] from a list of (Entity, PathPattern).
	/// Returns an error if there are conflicting paths.
	pub fn from_endpoints(
		endpoints: Vec<(Entity, PathPattern, ParamsPattern)>,
	) -> Result<Self> {
		#[derive(Default)]
		struct Node {
			children: HashMap<String, Node>,
			endpoint: Option<Entity>,
			params: Option<ParamsPattern>,
			/// Track if this node represents a static segment for conflict detection
			is_static: Option<bool>,
		}

		let mut root = Node::default();

		// build tree and detect conflicts
		for (ent, pattern, params) in &endpoints {
			let segments = pattern.iter().cloned().collect::<Vec<_>>();
			let mut node = &mut root;

			for (idx, seg) in segments.iter().enumerate() {
				let is_last = idx == segments.len() - 1;
				let seg_is_static = seg.is_static();
				// Use annotated string as key to distinguish different segment types
				let key = seg.to_string_annotated();

				// check for conflicts at this level
				for (existing_key, existing_node) in &node.children {
					let existing_is_static =
						existing_node.is_static.unwrap_or(true);

					// conflict if we have different dynamic segments at same level
					if existing_key != &key
						&& !seg_is_static && !existing_is_static
					{
						bevybail!(
							"Path conflict: Cannot have multiple dynamic/greedy segments at same level. \
							Found '{}' and '{}' at the same position",
							existing_key,
							key
						);
					}

					// conflict if mixing static with dynamic at same level
					if existing_key != &key
						&& (seg_is_static != existing_is_static)
					{
						bevybail!(
							"Path conflict: Cannot mix static and dynamic segments at same level. \
							Found '{}' and '{}'",
							existing_key,
							key
						);
					}
				}

				node = node.children.entry(key).or_insert_with(|| Node {
					is_static: Some(seg_is_static),
					endpoint: None,
					params: None,
					children: default(),
				});

				if is_last {
					if node.endpoint.is_some() {
						bevybail!(
							"Duplicate endpoint: Multiple endpoints defined for path '{}'",
							pattern.annotated_route_path()
						);
					}
					node.endpoint = Some(*ent);
					node.params = Some(params.clone());
				}
			}

			// handle root path
			if segments.is_empty() {
				if node.endpoint.is_some() {
					bevybail!(
						"Duplicate endpoint: Multiple endpoints defined for path '/'"
					);
				}
				node.endpoint = Some(*ent);
				node.params = Some(params.clone());
			}
		}

		// recursively build EndpointTree from Node
		fn build_tree(
			pattern: PathPattern,
			params: ParamsPattern,
			node: &Node,
		) -> EndpointTree {
			let mut children: Vec<EndpointTree> = node
				.children
				.iter()
				.map(|(key, child_node)| {
					// Parse the annotated key back into a segment
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

			EndpointTree {
				pattern,
				params: node.params.clone().unwrap_or(params),
				endpoint: node.endpoint,
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

	/// Returns all endpoint paths in the tree
	pub fn flatten(&self) -> Vec<PathPattern> {
		let mut patterns = Vec::new();
		fn inner(patterns: &mut Vec<PathPattern>, node: &EndpointTree) {
			if node.endpoint.is_some() {
				patterns.push(node.pattern.clone());
			}
			for child in node.children.iter() {
				inner(patterns, child);
			}
		}
		inner(&mut patterns, self);
		patterns
	}
}

impl std::fmt::Display for EndpointTree {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		fn inner(
			node: &EndpointTree,
			f: &mut std::fmt::Formatter<'_>,
		) -> std::fmt::Result {
			if node.endpoint.is_some() {
				writeln!(f, "{}", node.pattern.annotated_route_path())?;
				for param in node.params.iter() {
					writeln!(f, "  {}", param)?;
				}
			}
			for child in &node.children {
				inner(child, f)?;
			}
			Ok(())
		}
		inner(self, f)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn endpoint_tree_detects_duplicates() {
		let mut world = World::new();
		let ent1 = world.spawn_empty().id();
		let ent2 = world.spawn_empty().id();

		let endpoints = vec![
			(
				ent1,
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("foo"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				ent2,
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("foo"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
		];

		let result = EndpointTree::from_endpoints(endpoints);
		result
			.unwrap_err()
			.to_string()
			.contains("Duplicate endpoint")
			.xpect_true();
	}

	#[test]
	fn endpoint_tree_detects_dynamic_conflicts() {
		let mut world = World::new();
		let ent1 = world.spawn_empty().id();
		let ent2 = world.spawn_empty().id();

		let endpoints = vec![
			(
				ent1,
				PathPattern::from_segments(vec![
					PathPatternSegment::dynamic_required("foo"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				ent2,
				PathPattern::from_segments(vec![
					PathPatternSegment::dynamic_required("bar"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
		];

		let result = EndpointTree::from_endpoints(endpoints);
		result
			.unwrap_err()
			.to_string()
			.contains("Path conflict")
			.xpect_true();
	}

	#[test]
	fn endpoint_tree_detects_static_dynamic_mix() {
		let mut world = World::new();
		let ent1 = world.spawn_empty().id();
		let ent2 = world.spawn_empty().id();

		let endpoints = vec![
			(
				ent1,
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("foo"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				ent2,
				PathPattern::from_segments(vec![
					PathPatternSegment::dynamic_required("bar"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
		];

		let result = EndpointTree::from_endpoints(endpoints);
		result
			.unwrap_err()
			.to_string()
			.contains("Path conflict")
			.xpect_true();
	}

	#[test]
	fn endpoint_tree_allows_different_static_paths() {
		let mut world = World::new();
		let ent1 = world.spawn_empty().id();
		let ent2 = world.spawn_empty().id();
		let ent3 = world.spawn_empty().id();

		let endpoints = vec![
			(
				ent1,
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("foo"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				ent2,
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("bar"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				ent3,
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("foo"),
					PathPatternSegment::static_segment("bar"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
		];

		let tree = EndpointTree::from_endpoints(endpoints).unwrap();
		tree.flatten().len().xpect_eq(3);
	}

	#[test]
	fn endpoint_tree_greedy_conflict() {
		let mut world = World::new();
		let ent1 = world.spawn_empty().id();
		let ent2 = world.spawn_empty().id();

		let endpoints = vec![
			(
				ent1,
				PathPattern::from_segments(vec![
					PathPatternSegment::one_or_more("foo"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				ent2,
				PathPattern::from_segments(vec![
					PathPatternSegment::one_or_more("bar"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
		];

		let result = EndpointTree::from_endpoints(endpoints);
		result
			.unwrap_err()
			.to_string()
			.contains("Path conflict")
			.xpect_true();
	}

	#[test]
	#[cfg(feature = "_sweet_runner")]
	fn complex() {
		let mut world = World::new();
		let ent1 = world.spawn_empty().id();
		let ent2 = world.spawn_empty().id();
		let ent3 = world.spawn_empty().id();
		let ent4 = world.spawn_empty().id();

		// valid tree with mixed static and dynamic paths at different levels
		let endpoints = vec![
			(
				ent1,
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("api"),
				])
				.unwrap(),
				ParamsPattern::from_metas(vec![
					ParamMeta::new("verbose", ParamValue::Flag)
						.with_short('v')
						.with_description("Enable verbose output"),
				])
				.unwrap(),
			),
			(
				ent2,
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("api"),
					PathPatternSegment::dynamic_required("id"),
				])
				.unwrap(),
				ParamsPattern::from_metas(vec![
					ParamMeta::new("verbose", ParamValue::Flag)
						.with_short('v')
						.with_description("Enable verbose output"),
					ParamMeta::new("format", ParamValue::Single)
						.with_short('f')
						.with_description("Output format")
						.required(),
				])
				.unwrap(),
			),
			(
				ent3,
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("users"),
					PathPatternSegment::dynamic_required("userId"),
				])
				.unwrap(),
				ParamsPattern::from_metas(vec![
					ParamMeta::new("tags", ParamValue::Multiple)
						.with_description("User tags"),
				])
				.unwrap(),
			),
			(
				ent4,
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("docs"),
					PathPatternSegment::one_or_more("path"),
				])
				.unwrap(),
				ParamsPattern::from_metas(vec![]).unwrap(),
			),
		];

		EndpointTree::from_endpoints(endpoints)
			.unwrap()
			.to_string()
			.xpect_snapshot();
	}

	#[test]
	fn endpoint_tree_rejects_dynamic_static_same_level() {
		let mut world = World::new();
		let ent1 = world.spawn_empty().id();
		let ent2 = world.spawn_empty().id();

		// cannot have /api/:id and /api/users at same level
		let endpoints = vec![
			(
				ent1,
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("api"),
					PathPatternSegment::dynamic_required("id"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				ent2,
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("api"),
					PathPatternSegment::static_segment("users"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
		];

		let result = EndpointTree::from_endpoints(endpoints);
		result
			.unwrap_err()
			.to_string()
			.contains("Path conflict")
			.xpect_true();
	}
}
