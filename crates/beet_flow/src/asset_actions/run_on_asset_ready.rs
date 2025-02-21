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
	/// The action to trigger.
	pub trigger: OnRunAction<P>,
}

impl<A: Asset, P: Default> RunOnAssetReady<A, P> {
	/// Create a new [`RunOnAssetReady`] action with a default payload.
	pub fn new(handle: Handle<A>) -> Self {
		Self {
			handle,
			trigger: Default::default(),
		}
	}
}
impl<A: Asset, P> RunOnAssetReady<A, P> {
	/// Create a new [`RunOnAssetReady`] action with a payload.
	pub fn new_with_trigger(
		handle: Handle<A>,
		trigger: OnRunAction<P>,
	) -> Self {
		Self { handle, trigger }
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
							.trigger(run_on_ready.trigger.clone());
					}
				}
			}
			_ => {}
		}
	}
}
