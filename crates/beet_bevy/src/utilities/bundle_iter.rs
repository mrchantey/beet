use bevy::ecs::bundle::BundleEffect;
use bevy::ecs::bundle::DynamicBundle;
use bevy::prelude::*;

/// Like [`SpawnIter`] but for bundles, calling [`EntityWorldMut::insert`].
pub struct BundleIter<T>(pub T);

impl<T> BundleIter<T> {
	/// Create a new [`BundleIter`] effect.
	pub fn new(iter: T) -> Self { Self(iter) }
}

impl<T, B> BundleEffect for BundleIter<T>
where
	T: Send + Sync + Iterator<Item = B>,
	B: Bundle,
{
	fn apply(self, entity: &mut EntityWorldMut) {
		for bundle in self.0 {
			entity.insert(bundle);
		}
	}
}

impl<T: 'static + Send + Sync + Iterator<Item = B>, B: Bundle> DynamicBundle
	for BundleIter<T>
{
	type Effect = Self;
	fn get_components(
		self,
		_func: &mut impl FnMut(
			bevy::ecs::component::StorageType,
			bevy::ptr::OwningPtr<'_>,
		),
	) -> Self::Effect {
		self
	}
}

unsafe impl<T: 'static + Send + Sync + Iterator<Item = B>, B: Bundle> Bundle
	for BundleIter<T>
{
	fn component_ids(
		_components: &mut bevy::ecs::component::ComponentsRegistrator,
		_ids: &mut impl FnMut(bevy::ecs::component::ComponentId),
	) {
	}

	fn get_component_ids(
		_components: &bevy::ecs::component::Components,
		_ids: &mut impl FnMut(Option<bevy::ecs::component::ComponentId>),
	) {
	}

	fn register_required_components(
		_components: &mut bevy::ecs::component::ComponentsRegistrator,
		_required_components: &mut bevy::ecs::component::RequiredComponents,
	) {
	}
}
