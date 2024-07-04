use beet::prelude::AppReady;
use bevy::prelude::*;
use bevy::utils::HashSet;
use std::ops::Deref;
use std::ops::DerefMut;

#[derive(Debug, Default, Clone, Resource)]
pub struct ReadyOnAssetLoad {
	pub lookup: HashSet<String>,
	loaded: usize,
}

impl ReadyOnAssetLoad {
	pub fn insert(&mut self, path: impl Into<String>) {
		self.lookup.insert(path.into());
	}
}

impl Deref for ReadyOnAssetLoad {
	type Target = HashSet<String>;
	fn deref(&self) -> &Self::Target { &self.lookup }
}
impl DerefMut for ReadyOnAssetLoad {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.lookup }
}

pub fn ready_on_asset_load<A: Asset>(
	mut ready: ResMut<ReadyOnAssetLoad>,
	mut asset_events: EventReader<AssetEvent<A>>,
	mut ready_events: EventWriter<AppReady>,
	query: Query<&Handle<A>>,
) {
	for ev in asset_events.read() {
		match ev {
			AssetEvent::LoadedWithDependencies { id } => {
				for handle in query.iter() {
					if handle.id() == *id {
						if let Some(path) = handle.path() {
							if ready.lookup.contains(&path.to_string()) {
								ready.loaded += 1;
								if ready.loaded == ready.lookup.len() {
									ready_events.send(AppReady);
								}
							}
						}
					}
					// ready_events.send(beet_net::prelude::AppReady);
				}
			}
			_ => {}
		}
	}
}
