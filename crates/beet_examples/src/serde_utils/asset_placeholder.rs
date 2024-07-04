use bevy::prelude::*;
use bevy::utils::HashMap;
use std::marker::PhantomData;

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct AssetPlaceholder<A> {
	pub path: String,
	#[reflect(ignore)]
	phantom: PhantomData<A>,
}

impl<A> AssetPlaceholder<A> {
	pub fn new(path: impl Into<String>) -> Self {
		Self {
			path: path.into(),
			phantom: PhantomData,
		}
	}
}

#[derive(Debug, Default, Clone, Resource, Reflect)]
pub struct AssetPlaceholderLookup<A: Asset>(pub HashMap<String, Handle<A>>);

impl<A: Asset> AssetPlaceholderLookup<A> {
	pub fn get_or_create(
		&mut self,
		asset_server: &mut ResMut<AssetServer>,
		path: &String,
	) -> Handle<A> {
		self.0
			.entry(path.clone())
			.or_insert_with(|| asset_server.load(path))
			.clone()
	}
}

#[derive(Debug)]
pub struct AssetPlaceholderPlugin<T>(PhantomData<T>);

impl<T> Default for AssetPlaceholderPlugin<T> {
	fn default() -> Self { Self(PhantomData) }
}

impl<A: Asset> Plugin for AssetPlaceholderPlugin<A> {
	fn build(&self, app: &mut App) {
		app.insert_resource(AssetPlaceholderLookup::<A>(Default::default()))
			.add_systems(PreUpdate, init_asset::<A>)
			.register_type::<AssetPlaceholder<A>>();
	}
}


fn init_asset<A: Asset>(
	mut commands: Commands,
	mut lookup: ResMut<AssetPlaceholderLookup<A>>,
	mut asset_server: ResMut<AssetServer>,
	query: Query<(Entity, &AssetPlaceholder<A>), Added<AssetPlaceholder<A>>>,
) {
	for (entity, placeholder) in query.iter() {
		let handle = lookup.get_or_create(&mut asset_server, &placeholder.path);
		commands
			.entity(entity)
			.insert(handle)
			.remove::<AssetPlaceholder<A>>();
		// placeholder used for readyOnPlaceholder
	}
}
