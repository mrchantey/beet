use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::Actions;
use beet_net::prelude::*;

/// Collects all canonical endpoints in an application and arranges them into a tree structure.
///
/// This serves as a validation step, ensuring there is only a single canonical endpoint for
/// any given path pattern. It also detects conflicts between dynamic and greedy
/// segments that would cause ambiguous routing.
///
/// Non-canonical endpoints are excluded from the tree to avoid conflicts with canonical routes.
/// Use `EndpointBuilder::non_canonical()` for fallback endpoints that shouldn't conflict.
///
/// ## Validation Rules
/// - Only one canonical endpoint per exact path pattern
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
/// world.spawn(flow_exchange(|| {
///   (
/// 		InfallibleSequence, children![
///       EndpointBuilder::get().with_path("api"),
///       EndpointBuilder::get().with_path("users/:id"),
///       EndpointBuilder::get().with_path("docs/*path"),
///     ]
/// 	)
/// }));
///
/// ```
#[derive(Debug, Clone, Component)]
pub struct EndpointTree {
	/// The path pattern for this node
	pub pattern: PathPattern,
	/// The params pattern for this node
	pub params: ParamsPattern,
	/// The [`Endpoint`] at this exact path, if any
	pub endpoint: Option<Endpoint>,
	/// Child nodes in the tree
	pub children: Vec<EndpointTree>,
}

impl EndpointTree {
	/// Builds a list of [`Endpoint`] by spawning a bundle func
	/// in the given world and collecting all canonical endpoints from its descendants,
	/// then despawning the entity. Non-canonical endpoints are excluded.
	///
	/// This spawns the bundle directly and traverses all descendants looking for
	/// [`Endpoint`] components, without requiring the full exchange machinery.
	pub fn endpoints_from_bundle_func(
		world: &mut World,
		func: impl BundleFunc,
	) -> Result<Vec<Endpoint>> {
		// Spawn the bundle directly and collect endpoints from its descendants
		let root = world.spawn(func.bundle_func()).id();

		let endpoints = world
			.run_system_cached_with::<_, Result<Vec<Endpoint>>, _, _>(
				|root: In<Entity>,
				 children: Query<&Children>,
				 endpoints: Query<&Endpoint>| {
					children
						.iter_descendants_inclusive(*root)
						.filter_map(|entity| {
							endpoints
								.get(entity)
								.ok()
								.filter(|endpoint| endpoint.is_canonical())
								.cloned()
						})
						.collect::<Vec<_>>()
						.xok()
				},
				root,
			)??;

		world.despawn(root);
		endpoints.xok()
	}

	/// Get the canonical endpoints and entities for an already spawned root [`ExchangeSpawner::spawn`]
	/// Non-canonical endpoints are excluded.
	#[deprecated = "Left-over from ExchangeSpawner pattern"]
	pub fn endpoints_from_exchange(
		root: In<Entity>,
		actions: Query<&Actions>,
		children: Query<&Children>,
		endpoints: Query<&Endpoint>,
	) -> Result<Vec<(Entity, Endpoint)>> {
		let actions = actions.get(*root)?;
		assert_eq!(actions.len(), 1,);
		children
			.iter_descendants_inclusive(actions[0])
			.filter_map(|entity| {
				endpoints
					.get(entity)
					.ok()
					.filter(|endpoint| endpoint.is_canonical())
					.map(|endpoint| (entity, (*endpoint).clone()))
			})
			.collect::<Vec<_>>()
			.xok()
	}

	/// Builds an [`EndpointTree`] from a list of (Entity, Endpoint).
	/// Only canonical endpoints should be passed; non-canonical endpoints are typically
	/// filtered out before calling this method.
	/// Returns an error if there are conflicting paths.
	/// Builds an [`EndpointTree`] from a list of Endpoints.
	/// Only canonical endpoints should be passed; non-canonical endpoints are typically
	/// filtered out before calling this method.
	/// Returns an error if there are conflicting paths.
	pub fn from_endpoints(endpoints: Vec<Endpoint>) -> Result<Self> {
		#[derive(Default)]
		struct Node {
			children: HashMap<String, Node>,
			endpoint: Option<Endpoint>,
			params: Option<ParamsPattern>,
			/// Track if this node represents a static segment for conflict detection
			is_static: Option<bool>,
		}

		let mut root = Node::default();

		// build tree and detect conflicts
		for endpoint in &endpoints {
			let segments = endpoint.path().iter().cloned().collect::<Vec<_>>();
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
							"Duplicate endpoint: Multiple canonical endpoints defined for path '{}'. \
							Consider marking one as non-canonical with `.non_canonical()`",
							endpoint.path().annotated_route_path()
						);
					}
					node.endpoint = Some(endpoint.clone());
					node.params = Some(endpoint.params().clone());
				}
			}

			// handle root path
			if segments.is_empty() {
				if node.endpoint.is_some() {
					bevybail!(
						"Duplicate endpoint: Multiple canonical endpoints defined for path '/'. \
						Consider marking one as non-canonical with `.non_canonical()`"
					);
				}
				node.endpoint = Some(endpoint.clone());
				node.params = Some(endpoint.params().clone());
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
				endpoint: node.endpoint.clone(),
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

	/// Helper to create canonical endpoints for testing
	fn create_endpoints(
		endpoints: Vec<(PathPattern, ParamsPattern)>,
	) -> Vec<Endpoint> {
		create_endpoints_with_canonical(endpoints, true)
	}

	/// Helper to create endpoints with specified canonical flag
	fn create_endpoints_with_canonical(
		endpoints: Vec<(PathPattern, ParamsPattern)>,
		is_canonical: bool,
	) -> Vec<Endpoint> {
		endpoints
			.into_iter()
			.map(|(path, params)| {
				Endpoint::new(path, params, None, None, is_canonical)
			})
			.collect()
	}


	#[test]
	fn endpoint_tree_detects_duplicates() {
		let endpoints = create_endpoints(vec![
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("foo"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("foo"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
		]);

		EndpointTree::from_endpoints(endpoints)
			.unwrap_err()
			.to_string()
			.contains("Duplicate endpoint")
			.xpect_true();
	}

	#[test]
	fn endpoint_tree_detects_dynamic_conflicts() {
		let endpoints = create_endpoints(vec![
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::dynamic_required("foo"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::dynamic_required("bar"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
		]);

		let result = EndpointTree::from_endpoints(endpoints);
		result
			.unwrap_err()
			.to_string()
			.contains("Path conflict")
			.xpect_true();
	}

	#[test]
	fn endpoint_tree_detects_static_dynamic_mix() {
		let endpoints = create_endpoints(vec![
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("foo"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::dynamic_required("bar"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
		]);

		let result = EndpointTree::from_endpoints(endpoints);
		result
			.unwrap_err()
			.to_string()
			.contains("Path conflict")
			.xpect_true();
	}

	#[test]
	fn endpoint_tree_allows_different_static_paths() {
		let endpoints = create_endpoints(vec![
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("foo"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("bar"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("foo"),
					PathPatternSegment::static_segment("bar"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
		]);

		let tree = EndpointTree::from_endpoints(endpoints).unwrap();
		tree.flatten().len().xpect_eq(3);
	}

	#[test]
	fn endpoint_tree_greedy_conflict() {
		let endpoints = create_endpoints(vec![
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::one_or_more("foo"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::one_or_more("bar"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
		]);

		let result = EndpointTree::from_endpoints(endpoints);
		result
			.unwrap_err()
			.to_string()
			.contains("Path conflict")
			.xpect_true();
	}

	#[test]
	fn complex() {
		let endpoints = create_endpoints(vec![
			(
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
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("docs"),
					PathPatternSegment::one_or_more("path"),
				])
				.unwrap(),
				ParamsPattern::from_metas(vec![]).unwrap(),
			),
		]);

		EndpointTree::from_endpoints(endpoints)
			.unwrap()
			.to_string()
			.xpect_snapshot();
	}

	#[test]
	fn endpoint_tree_rejects_dynamic_static_same_level() {
		let endpoints = create_endpoints(vec![
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("api"),
					PathPatternSegment::dynamic_required("id"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
			(
				PathPattern::from_segments(vec![
					PathPatternSegment::static_segment("api"),
					PathPatternSegment::static_segment("users"),
				])
				.unwrap(),
				ParamsPattern::default(),
			),
		]);

		let result = EndpointTree::from_endpoints(endpoints);
		result
			.unwrap_err()
			.to_string()
			.contains("Path conflict")
			.xpect_true();
	}

	#[test]
	fn non_canonical_endpoints_excluded() {
		// Create two endpoints at the same path, one canonical, one not
		let path_pattern = PathPattern::from_segments(vec![
			PathPatternSegment::static_segment("foo"),
		])
		.unwrap();

		let canonical_endpoints = create_endpoints_with_canonical(
			vec![(path_pattern.clone(), ParamsPattern::default())],
			true,
		);
		let non_canonical_endpoints = create_endpoints_with_canonical(
			vec![(path_pattern, ParamsPattern::default())],
			false,
		);

		// Filter non-canonical as would happen in real usage
		let canonical_only: Vec<_> = canonical_endpoints
			.xtend(non_canonical_endpoints)
			.into_iter()
			.filter(|endpoint| endpoint.is_canonical())
			.collect();

		// Should succeed - only one canonical endpoint at /foo
		let tree = EndpointTree::from_endpoints(canonical_only).unwrap();
		tree.flatten().len().xpect_eq(1);
	}
}
