use crate::prelude::*;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;


pub trait IntoRouteLayer<M> {
	fn into_route_layer(self) -> RouteLayer;
}


/// Plugins added to routes or their ancestors, to be added to the route app.
#[derive(Clone, Component, Deref)]
pub struct RouteLayer(ClonePluginContainer);


impl RouteLayer {
	pub fn new<M>(layer: impl IntoRouteLayer<M>) -> Self {
		layer.into_route_layer()
	}
}


pub struct PluginIntoRouteLayerMarker;

impl<P> IntoRouteLayer<PluginIntoRouteLayerMarker> for P
where
	P: 'static + Clone + Plugin,
{
	fn into_route_layer(self) -> RouteLayer {
		RouteLayer(ClonePluginContainer::new(self))
	}
}
pub struct ScheduleIntoRouteLayerMarker;

impl<S, M> IntoRouteLayer<(ScheduleIntoRouteLayerMarker, M)> for S
where
	S: 'static + Send + Sync + Clone + IntoScheduleConfigs<ScheduleSystem, M>,
{
	fn into_route_layer(self) -> RouteLayer {
		use std::sync::Arc;
		let this = Arc::new(self);
		RouteLayer(ClonePluginContainer::new({
			let this = Arc::clone(&this);
			move |app: &mut App| {
				app.add_systems(Update, (*this).clone());
			}
		}))
	}
}


#[cfg(test)]
mod test {
	// use crate::prelude::*;
	use bevy::prelude::*;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		let app = App::new();
		// app.add_systems(schedule, systems)
		// let mut world = World::new();
		// world.add_systems(U

		// expect(true).to_be_false();
	}
}
