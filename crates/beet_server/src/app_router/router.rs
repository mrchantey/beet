use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
/// Collection of systems for collecting and running and route handlers
pub struct BeetRouter;



impl BeetRouter {
	pub fn collect_routes(world: &mut World) -> Result<Vec<RouteInstance>> {
		world
			.query_once::<&RouteInfo>()
			.into_iter()
			.map(|info| info.clone())
			.collect::<Vec<_>>()
			.into_iter()
			.map(|route_info| {
				let val = world.run_system_cached_with(
					RouteInstance::from_info,
					route_info.clone(),
				)??;
				Ok(val)
			})
			.collect()
	}
	#[cfg(test)]
	pub async fn oneshot(
		world: &mut World,
		route: impl Into<RouteInfo>,
	) -> Result<Response> {
		let route = route.into();
		world
			.run_system_cached_with(RouteInstance::from_info, route.clone())??
			.call(route.into())
			.await?
			.xok()
	}

	/// For testing, collect all routes and return the base route as a string
	#[cfg(test)]
	pub async fn oneshot_str(
		world: &mut World,
		route: impl Into<RouteInfo>,
	) -> Result<String> {
		Self::oneshot(world, route)
			.await?
			.xmap(|res| res.body_str())
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

		BeetRouter::oneshot_str(&mut world, "/")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hello world!");
	}
}
