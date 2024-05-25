use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

/// Base trait for any [`Component`], [`Resource`], or [`Event`] that can be replicated.
pub trait ReplicateType<Marker>: 'static + Send + Sync {
	fn register(
		registrations: &mut ReplicateRegistry,
		direction: ReplicateDirection,
	);
	fn outgoing_systems() -> SystemConfigs;
}

impl ReplicateType<()> for () {
	fn register(
		_registrations: &mut ReplicateRegistry,
		_direction: ReplicateDirection,
	) {
	}
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
		replicate::<T, ReplicateComponentMarker>(
			self,
			ReplicateDirection::Both,
		);
		self
	}
	fn replicate_with<T: ReplicateType<ReplicateComponentMarker>>(
		&mut self,
		direction: ReplicateDirection,
	) -> &mut Self {
		replicate::<T, ReplicateComponentMarker>(self, direction);
		self
	}
	fn replicate_resource_incoming<
		T: ReplicateType<ReplicateResourceMarker>,
	>(
		&mut self,
	) -> &mut Self {
		replicate::<T, ReplicateResourceMarker>(
			self,
			ReplicateDirection::Incoming,
		);
		self
	}
	fn replicate_resource_outgoing<
		T: ReplicateType<ReplicateResourceMarker>,
	>(
		&mut self,
	) -> &mut Self {
		replicate::<T, ReplicateResourceMarker>(
			self,
			ReplicateDirection::Outgoing,
		);
		self
	}

	fn replicate_event_incoming<T: ReplicateType<ReplicateEventMarker>>(
		&mut self,
	) -> &mut Self {
		replicate::<T, ReplicateEventMarker>(
			self,
			ReplicateDirection::Incoming,
		);
		self
	}
	fn replicate_event_outgoing<T: ReplicateType<ReplicateEventMarker>>(
		&mut self,
	) -> &mut Self {
		replicate::<T, ReplicateEventMarker>(
			self,
			ReplicateDirection::Outgoing,
		);
		self
	}
}

pub fn replicate<T: ReplicateType<M>, M>(
	app: &mut App,
	direction: ReplicateDirection,
) {
	app.init_resource::<ReplicateRegistry>();
	let mut registrations = app.world_mut().resource_mut::<ReplicateRegistry>();
	T::register(&mut registrations, direction);
	if direction.is_outgoing() {
		app.add_systems(
			Update,
			T::outgoing_systems().in_set(MessageOutgoingSet),
		);
	}
}
