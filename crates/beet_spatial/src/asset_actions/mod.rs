//! Actions related to asset events.
mod insert_on_asset_event;
pub use self::insert_on_asset_event::*;
mod reflected_asset_event;
pub use self::reflected_asset_event::*;
use beet_flow::prelude::*;
use beet_core::prelude::*;

/// Adds systems for a [`InsertOnAssetEvent`] action.
pub fn insert_on_asset_event_plugin<T: Component + Clone, A: Asset>(
	app: &mut App,
) {
	app.add_systems(
		Update,
		(
			insert_on_asset_status::<T, A>,
			insert_on_asset_event::<T, A>,
		)
			.in_set(TickSet),
	);
}
