use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
/// Collection of systems for collecting and running and route handlers
pub struct BeetRouter;


impl BeetRouter {
	pub fn route_entity(
		info: In<impl Into<RouteInfo>>,
		query: Query<(Entity, &RouteInfo)>,
	) -> AppResult<Entity> {
		let route_info = info.0.into();
		query
			.iter()
			.find(|(_, r)| r == &&route_info)
			.map(|(entity, _)| entity)
			.ok_or_else(|| {
				AppError::not_found(format!("Route not found: {}", route_info))
			})
	}
	pub fn route_instance(
		info: In<impl Into<RouteInfo>>,
		world: &mut World,
	) -> Result<RouteInstance> {
		let info = info.0.into();
		let route = world
			.run_system_cached_with(Self::route_entity, info.clone())??;
		world
			.run_system_cached_with(RouteInstance::from_entity, route)?
			.xok()
	}

	pub fn collect_routes(world: &mut World) -> Result<Vec<RouteInstance>> {
		world
			.query_once::<&RouteInfo>()
			.into_iter()
			.map(|info| info.clone())
			.collect::<Vec<_>>()
			.into_iter()
			.map(|route_info| {
				let val = world.run_system_cached_with(
					Self::route_instance,
					route_info.clone(),
				)??;
				Ok(val)
			})
			.collect()
	}

	/// For testing, collect all routes and return the base route as a string
	#[cfg(test)]
	pub async fn route_str(
		world: &mut World,
		route: impl Into<RouteInfo>,
	) -> Result<String> {
		let route = route.into();
		world
			.run_system_cached_with(Self::route_instance, route.clone())??
			.call(route.into())
			.await?
			.xmap(|res| res.body_str().unwrap())
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn beet_route_works() {
		let mut world = World::new();
		world.spawn(children![(
			RouteInfo::get("/"),
			RouteHandler::new(|mut commands: Commands| {
				commands.insert_resource("hello world!".into_response());
			})
		),]);

		BeetRouter::route_str(&mut world, "/")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hello world!");
	}
}
