//! Actions related to the loading and handling of bevy assets.

use std::marker::PhantomData;

use crate::prelude::*;
use beet_core::prelude::*;

/// A plugin that registers the [`RunOnAssetReady`] system.
pub struct RunOnAssetReadyPlugin<A: Asset>(PhantomData<A>);

impl<A: Asset> Default for RunOnAssetReadyPlugin<A> {
	fn default() -> Self { Self(PhantomData) }
}

impl<A: Asset> Plugin for RunOnAssetReadyPlugin<A> {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, run_on_asset_ready::<A>);
	}
}

/// An action that will trigger [`Run`] when an asset with
/// the provided handle is loaded.
/// ## Warning
/// The [`RunOnAssetReadyPlugin`] must be registered with matching
/// generic parameters for this action to work.
#[derive(Debug, Component)]
pub struct RunOnAssetReady<A: Asset> {
	/// The handle of the asset to wait for.
	pub handle: Handle<A>,
}

impl<A: Asset> RunOnAssetReady<A> {
	/// Create a new [`RunOnAssetReady`] action with the provided handle.
	pub fn new(handle: Handle<A>) -> Self { Self { handle } }
}

fn run_on_asset_ready<A: Asset>(
	mut asset_events: MessageReader<AssetEvent<A>>,
	mut commands: Commands,
	query: Query<(Entity, &RunOnAssetReady<A>)>,
) {
	for ev in asset_events.read() {
		match ev {
			AssetEvent::LoadedWithDependencies { id } => {
				for (entity, run_on_ready) in query.iter() {
					if run_on_ready.handle.id() == *id {
						commands
							.entity(entity)
							.remove::<RunOnAssetReady<A>>()
							.trigger_payload(GetOutcome);
					}
				}
			}
			_ => {}
		}
	}
}
