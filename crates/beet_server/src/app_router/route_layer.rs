use crate::prelude::*;
use bevy::app::Plugins;
use bevy::ecs::system::IntoSystem;
use bevy::prelude::*;


/// Plugins added to routes or their ancestors, to be added to the route app.
/// This type accepts any valid [`Plugins`] or [`System`]
#[derive(Clone, Component, Deref)]
pub struct RouteLayer(ClonePluginContainer);


impl RouteLayer {
	pub fn new<P, M>(plugins: P) -> Self
	where
		P: 'static + Send + Sync + Clone + Plugins<M>,
	{
		RouteLayer(ClonePluginContainer::new(move |app: &mut App| {
			plugins.clone().add_to_app(app);
		}))
	}

	pub fn before_route<S, M>(system: S) -> Self
	where
		S: 'static + Send + Sync + Clone + IntoSystem<(), (), M>,
	{
		Self(ClonePluginContainer::new(move |app: &mut App| {
			app.add_systems(BeforeRoute, system.clone());
		}))
	}

	pub fn after_route<S, M>(system: S) -> Self
	where
		S: 'static + Send + Sync + Clone + IntoSystem<(), (), M>,
	{
		Self(ClonePluginContainer::new(move |app: &mut App| {
			app.add_systems(AfterRoute, system.clone());
		}))
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::http_resources::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn beet_route_works() {
		let mut world = World::new();
		world.spawn((
			RouteLayer::before_route(|mut req: ResMut<Request>| {
				req.set_body("jimmy");
			}),
			children![(
				RouteInfo::get("/"),
				RouteHandler::new(
					|req: Res<Request>, mut commands: Commands| {
						let body = req.body_str().unwrap_or_default();
						commands.insert_resource(
							format!("hello {}", body).into_response(),
						);
					}
				)
			),],
		));

		world
			.run_system_cached_with(collect_routes, BeetRouter::default())
			.unwrap()
			.unwrap()
			.oneshot("/")
			.await
			.unwrap()
			.xmap(|res| res.body_str().unwrap())
			.xpect()
			.to_be("hello jimmy".to_string());
	}
}
