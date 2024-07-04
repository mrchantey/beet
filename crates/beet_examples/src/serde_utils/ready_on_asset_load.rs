use beet::prelude::AppReady;
use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct AssetLoadBlockAppReady;

pub struct ReadyOnAssetLoadPlugin<A: Asset>(PhantomData<A>);

impl<A: Asset> Default for ReadyOnAssetLoadPlugin<A> {
	fn default() -> Self { Self(PhantomData) }
}

impl<A: Asset> Plugin for ReadyOnAssetLoadPlugin<A> {
	fn build(&self, app: &mut App) {
		app.add_systems(PreUpdate, ready_on_asset_load::<A>)
			.register_type::<AssetLoadBlockAppReady>();
	}
}

pub fn ready_on_asset_load<A: Asset>(
	mut asset_events: EventReader<AssetEvent<A>>,
	mut ready_events: EventWriter<AppReady>,
	mut commands: Commands,
	query: Query<(Entity, &Handle<A>), With<AssetLoadBlockAppReady>>,
	all_blocks: Query<Entity, With<AssetLoadBlockAppReady>>,
) {
	for ev in asset_events.read() {
		match ev {
			AssetEvent::LoadedWithDependencies { id } => {
				for (entity, handle) in query.iter() {
					if handle.id() == *id {
						commands
							.entity(entity)
							.remove::<AssetLoadBlockAppReady>();
						if all_blocks.iter().count() == 1 {
							ready_events.send(AppReady);
						}
					}
				}
			}
			_ => {}
		}
	}
}
