use super::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use bevy::asset::LoadState;

/// Inserts the given component when a matching asset event is received.
/// This requires the entity to have a `Handle<A>`.
///
/// A long-running action: while [`Running`] the [`insert_on_asset_event`]
/// system listens for matching [`AssetEvent`]s (or matching initial load
/// state) and ends the run with [`Outcome::PASS`] once the event fires,
/// inserting `value` on the entity.
///
/// For each type the [`insert_on_asset_event_plugin`] must be registered.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[require(ContinueRun<(), Outcome>)]
#[reflect(Component)]
pub struct InsertOnAssetEvent<T: Component + Clone, A: Asset> {
	/// The component to insert.
	pub value: T,
	/// The asset event to match.
	pub asset_event: ReflectedAssetEvent<A>,
}

impl<T: Component + Clone, A: Asset> InsertOnAssetEvent<T, A> {
	/// Creates a new InsertOnAssetEvent.
	pub fn new(value: T, asset_event: AssetEvent<A>) -> Self {
		Self {
			value,
			asset_event: asset_event.into(),
		}
	}
	/// Creates a new InsertOnAssetEvent that matches the [AssetEvent::LoadedWithDependencies] event.
	pub fn loaded(value: T, handle: &Handle<A>) -> Self {
		Self::new(value, AssetEvent::LoadedWithDependencies {
			id: handle.id(),
		})
	}

	/// Checks if the given state matches the asset event.
	pub fn matches_load_state(&self, state: LoadState) -> bool {
		match (self.asset_event, state) {
			(ReflectedAssetEvent::Added { .. }, LoadState::Loaded) => true,
			(
				ReflectedAssetEvent::LoadedWithDependencies { .. },
				LoadState::Loaded,
			) => true,
			(ReflectedAssetEvent::Removed { .. }, LoadState::NotLoaded) => true,
			(_, _) => false,
		}
	}
}

/// Ends any [`Running`] [`InsertOnAssetEvent`] whose matching [`AssetEvent`]
/// has fired, inserting the stored value on the entity.
pub(crate) fn insert_on_asset_event<T: Component + Clone, A: Asset>(
	mut commands: Commands,
	mut asset_events: MessageReader<AssetEvent<A>>,
	query: Populated<
		(Entity, &InsertOnAssetEvent<T, A>),
		With<Running<Outcome>>,
	>,
) {
	for ev in asset_events.read() {
		for (entity, action) in query.iter() {
			let action_event: AssetEvent<A> = action.asset_event.into();
			if action_event == *ev {
				commands
					.entity(entity)
					.insert(action.value.clone())
					.queue(EndRun(Outcome::PASS));
			}
		}
	}
}

/// On entering the [`Running`] state, ends the run immediately if the
/// asset is already in a state matching `asset_event` (i.e. it loaded
/// before the action started).
pub(crate) fn insert_on_asset_status<T: Component + Clone, A: Asset>(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	query: Query<(Entity, &InsertOnAssetEvent<T, A>), Added<Running<Outcome>>>,
) {
	for (entity, action) in query.iter() {
		let id = asset_event_id(action.asset_event.into());
		let Some(state) = asset_server.get_load_state(id) else {
			continue;
		};
		if action.matches_load_state(state) {
			commands
				.entity(entity)
				.insert(action.value.clone())
				.queue(EndRun(Outcome::PASS));
		}
	}
}

fn asset_event_id<A: Asset>(ev: AssetEvent<A>) -> AssetId<A> {
	match ev {
		AssetEvent::Added { id } => id,
		AssetEvent::LoadedWithDependencies { id } => id,
		AssetEvent::Modified { id } => id,
		AssetEvent::Removed { id } => id,
		AssetEvent::Unused { id } => id,
	}
}
