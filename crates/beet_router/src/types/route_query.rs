use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;

#[derive(SystemParam)]
pub struct RouteQuery<'w, 's> {
	pub requests: AgentQuery<'w, 's, &'static RequestMeta>,
	pub parents: Query<'w, 's, &'static ChildOf>,
	pub path_partials: Query<'w, 's, &'static PathPartial>,
	pub params_partials: Query<'w, 's, &'static ParamsPartial>,
}

impl RouteQuery<'_, '_> {
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
}
