use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;

#[derive(SystemParam)]
pub struct RouteQuery<'w, 's> {
	commands: Commands<'w, 's>,
	pub requests: AgentQuery<'w, 's, &'static RequestMeta>,
	pub parents: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub path_partials: Query<'w, 's, &'static PathPartial>,
	pub params_partials: Query<'w, 's, &'static ParamsPartial>,
	endpoint_trees: Query<'w, 's, &'static EndpointTree>,
	endpoints: Query<'w, 's, &'static Endpoint>,
}

impl RouteQuery<'_, '_> {
	pub fn request_meta(&self, action: Entity) -> Result<&RequestMeta> {
		self.requests.get(action)?.xok()
	}

	pub fn path(&self, action: Entity) -> Result<&Vec<String>> {
		self.requests.get(action)?.path().xok()
	}
	pub fn method(&self, action: Entity) -> Result<HttpMethod> {
		self.requests.get(action)?.method().xok()
	}

	pub fn path_match(&self, action: Entity) -> Result<PathMatch> {
		let path = self.path(action)?;
		let pattern = PathPattern::collect(action, &self)?;
		pattern.parse_path(path)?.xok()
	}

	pub fn dyn_segment(&mut self, action: Entity, key: &str) -> Result<String> {
		self.path_match(action)?
			.dyn_map
			.get(key)
			.map(|key| key.clone())
			.ok_or_else(|| bevyhow!("key not found: {}", key))
	}

	pub async fn dyn_segment_async(
		action: AsyncEntity,
		key: &str,
	) -> Result<String> {
		let key = key.to_string();
		Self::with_async(action, move |query, entity| {
			query.dyn_segment(entity, &key)
		})
		.await
	}

	pub async fn with_async<F, O>(entity: AsyncEntity, func: F) -> O
	where
		F: 'static + Send + Sync + Clone + FnOnce(&mut RouteQuery, Entity) -> O,
		O: 'static + Send + Sync,
	{
		let id = entity.id();
		entity
			.world()
			.run_system_once_with(
				|In((func, id)): In<(F, Entity)>, mut query: RouteQuery| {
					func.clone()(&mut query, id)
				},
				(func, id),
			)
			.await
			.unwrap()
	}


	/// Get or build the endpoint tree for the given action,
	/// caching the result in the root of the tree
	pub fn endpoint_tree(&mut self, action: Entity) -> Result<EndpointTree> {
		let root = self.parents.root_ancestor(action);
		if let Ok(tree) = self.endpoint_trees.get(root) {
			tree.clone().xok()
		} else {
			let endpoints = self
				.children
				.iter_descendants_inclusive(root)
				.filter_map(|child| {
					self.endpoints
						.get(child)
						.ok()
						.map(|endpoint| (child, endpoint.clone()))
				})
				.collect();
			let tree = EndpointTree::from_endpoints(endpoints)?;
			self.commands.entity(root).insert(tree.clone());
			tree.xok()
		}
	}
}
