use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

/// Base trait for any [`Component`], [`Resource`], or [`Event`] that can be replicated.
pub trait ReplicateType<Marker>: 'static + Send + Sync {
	fn register(registrations: &mut ReplicateRegistry);
	fn outgoing_systems() -> SystemConfigs;
}

impl ReplicateType<()> for () {
	fn register(_registrations: &mut ReplicateRegistry) {}
	fn outgoing_systems() -> SystemConfigs { (|| ()).into_configs() }
}

pub struct ReplicateComponentMarker;
pub struct ReplicateResourceMarker;
pub struct ReplicateEventMarker;



#[extend::ext(name=AppExtReplicate)]
pub impl App {
	fn replicate<T: ReplicateType<ReplicateComponentMarker>>(
		&mut self,
	) -> &mut Self {
		replicate::<T, ReplicateComponentMarker>(self);
		self
	}
	fn replicate_resource<T: ReplicateType<ReplicateResourceMarker>>(
		&mut self,
	) -> &mut Self {
		replicate::<T, ReplicateResourceMarker>(self);
		self
	}
	fn replicate_event<T: ReplicateType<ReplicateEventMarker>>(
		&mut self,
	) -> &mut Self {
		replicate::<T, ReplicateEventMarker>(self);
		self
	}
}

pub fn replicate<T: ReplicateType<M>, M>(app: &mut App) {
	app.init_resource::<ReplicateRegistry>();
	let mut registrations = app.world_mut().resource_mut::<ReplicateRegistry>();
	T::register(&mut registrations);
	app.add_systems(Update, T::outgoing_systems().in_set(MessageOutgoingSet));
}
