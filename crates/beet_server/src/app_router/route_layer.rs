use crate::prelude::*;
use bevy::app::Plugins;
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
	/// Add a system to run after the route handler in the [`AfterRoute`] schedule.
	/// This method accepts either systems which are [`Clone`], or closures
	/// returing a system.
	pub fn before_route<M>(
		system: impl 'static + Send + Sync + CloneScheduleSystem<M>,
	) -> Self {
		Self(ClonePluginContainer::new(move |app: &mut App| {
			app.add_systems(BeforeRoute, system.clone().into_schedule_system());
		}))
	}

	/// Add a system to run after the route handler in the [`AfterRoute`] schedule.
	/// This method accepts either systems which are [`Clone`], or closures
	/// returing a system.
	pub fn after_route<M>(
		system: impl 'static + Send + Sync + CloneScheduleSystem<M>,
	) -> Self {
		Self(ClonePluginContainer::new(move |app: &mut App| {
			app.add_systems(AfterRoute, system.clone().into_schedule_system());
		}))
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

		BeetRouter::route_str(&mut world, "/")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hello jimmy");
	}
}
