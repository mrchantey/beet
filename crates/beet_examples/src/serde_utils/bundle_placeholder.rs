use bevy::prelude::*;
use std::marker::PhantomData;


#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct BundlePlaceholder<B: Bundle> {
	/// optionally override the transform of the bundle
	pub transform: Option<Transform>,
	phantom: PhantomData<B>,
}

impl<B: Bundle> BundlePlaceholder<B> {
	pub fn new() -> Self {
		Self {
			transform: None,
			phantom: PhantomData,
		}
	}
	pub fn new_with_transform(transform: Transform) -> Self {
		Self {
			transform: Some(transform),
			phantom: PhantomData,
		}
	}
}

#[derive(Debug, Default)]
pub struct BundlePlaceholderPlugin<B: Bundle>(PhantomData<B>);

impl<B: Bundle + Default> Plugin for BundlePlaceholder<B> {
	fn build(&self, app: &mut App) {
		app.add_systems(PreUpdate, init_bundle::<B>);
	}
}


fn init_bundle<B: Bundle + Default>(
	mut commands: Commands,
	query: Query<(Entity, &BundlePlaceholder<B>), Added<BundlePlaceholder<B>>>,
) {
	for (entity, placeholder) in query.iter() {
		commands
			.entity(entity)
			.insert(B::default())
			.remove::<BundlePlaceholder<B>>();
		if let Some(transform) = placeholder.transform {
			commands.entity(entity).insert(transform);
		}
	}
}
