//! Actions related to the loading and handling of bevy assets.

use std::marker::PhantomData;

use crate::prelude::*;
use bevy::prelude::*;

/// A plugin that registers the [`RunOnAssetReady`] system.
pub struct RunOnAssetReadyPlugin<A: Asset, P: RunPayload = ()>(
	PhantomData<(A, P)>,
);

impl<A: Asset, P: RunPayload> Default for RunOnAssetReadyPlugin<A, P> {
	fn default() -> Self { Self(PhantomData) }
}

impl<A: Asset, P: RunPayload> Plugin for RunOnAssetReadyPlugin<A, P> {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, run_on_asset_ready::<A, P>);
	}
}



/// An action that will trigger [`OnRun`] when an asset with
/// the provided handle is loaded.
/// ## Warning
/// The [`RunOnAssetReadyPlugin`] must be registered with matching
/// generic parameters for this action to work.
#[derive(Debug, Component)]
pub struct RunOnAssetReady<A: Asset, P = ()> {
	/// The handle of the asset to wait for.
	pub handle: Handle<A>,
	/// The payload to pass to the [`OnRunAction::payload`] event.
	pub payload: P,
	/// The entity to pass to [`OnRunAction::origin`].
	pub origin: Entity,
}

impl<A: Asset, P: Default> RunOnAssetReady<A, P> {
	/// Create a new [`RunOnAssetReady`] action with a default payload.
	pub fn new(handle: Handle<A>) -> Self {
		Self {
			handle,
			payload: Default::default(),
			origin: Entity::PLACEHOLDER,
		}
	}
	/// Create a new [`RunOnAssetReady`] action with a default payload.
	pub fn new_with_origin(handle: Handle<A>, origin: Entity) -> Self {
		Self {
			handle,
			payload: Default::default(),
			origin,
		}
	}
}
impl<A: Asset, P> RunOnAssetReady<A, P> {
	/// Create a new [`RunOnAssetReady`] action with a payload.
	pub fn new_with_payload(handle: Handle<A>, payload: P) -> Self {
		Self {
			handle,
			payload,
			origin: Entity::PLACEHOLDER,
		}
	}
}

fn run_on_asset_ready<A: Asset, P: RunPayload>(
	mut asset_events: EventReader<AssetEvent<A>>,
	mut commands: Commands,
	query: Query<(Entity, &RunOnAssetReady<A, P>)>,
) {
	for ev in asset_events.read() {
		match ev {
			AssetEvent::LoadedWithDependencies { id } => {
				for (entity, run_on_ready) in query.iter() {
					if run_on_ready.handle.id() == *id {
						commands
							.entity(entity)
							.remove::<RunOnAssetReady<A>>()
							.trigger(OnRunAction::new(
								entity,
								run_on_ready.origin,
								run_on_ready.payload.clone(),
							));
					}
				}
			}
			_ => {}
		}
	}
}
