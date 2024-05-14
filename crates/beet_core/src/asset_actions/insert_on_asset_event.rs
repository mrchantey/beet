use beet_ecs::prelude::*;
use bevy::asset::LoadState;
// use bevy::asset::LoadState;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

/// Inserts the given component when a matching asset event is received.
#[derive(PartialEq, Debug, Clone, Component, Reflect)]
#[reflect(Component, ActionMeta)]
pub struct InsertOnAssetEvent<T: GenericActionComponent, A: Asset> {
	pub value: T,
	pub asset_event: AssetEvent<A>,
}

impl<T: GenericActionComponent, A: Asset> ActionMeta
	for InsertOnAssetEvent<T, A>
{
	fn category(&self) -> ActionCategory { ActionCategory::Behavior }
}

impl<T: GenericActionComponent, A: Asset> ActionSystems
	for InsertOnAssetEvent<T, A>
{
	fn systems() -> SystemConfigs {
		(
			insert_on_asset_status::<T, A>,
			insert_on_asset_event::<T, A>,
		)
			.in_set(TickSet)
	}
}

impl<T: GenericActionComponent, A: Asset> InsertOnAssetEvent<T, A> {
	pub fn new(value: T, asset_event: AssetEvent<A>) -> Self {
		Self { value, asset_event }
	}
	pub fn loaded(value: T, handle: &Handle<A>) -> Self {
		Self::new(value, AssetEvent::LoadedWithDependencies {
			id: handle.id(),
		})
	}

	pub fn matches_load_state(&self, state: LoadState) -> bool {
		match (self.asset_event, state) {
			(AssetEvent::Added { .. }, LoadState::Loaded) => true,
			(AssetEvent::LoadedWithDependencies { .. }, LoadState::Loaded) => {
				true
			}
			(AssetEvent::Removed { .. }, LoadState::NotLoaded) => true,
			(_, _) => false,
		}
	}
}

fn insert_on_asset_event<T: GenericActionComponent, A: Asset>(
	mut commands: Commands,
	mut asset_events: EventReader<AssetEvent<A>>,
	query: Query<(Entity, &InsertOnAssetEvent<T, A>), With<Running>>,
) {
	for ev in asset_events.read() {
		for (entity, action) in query.iter() {
			if action.asset_event == *ev {
				commands.entity(entity).insert(action.value.clone());
			}
		}
	}
}
fn insert_on_asset_status<T: GenericActionComponent, A: Asset>(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	query: Query<(Entity, &InsertOnAssetEvent<T, A>), Added<Running>>,
) {
	for (entity, action) in query.iter() {
		let id = asset_event_id(action.asset_event);
		let Some(state) = asset_server.get_load_state(id) else {
			continue;
		};
		if action.matches_load_state(state) {
			commands.entity(entity).insert(action.value.clone());
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
