use beet_common_macros::ImplBundle;
use bevy::ecs::bundle::BundleEffect;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Like [`SpawnIter`] but for bundles, calling [`EntityWorldMut::insert`].
#[derive(ImplBundle)]
pub struct BundleIter<T, B>
where
	T: 'static + Send + Sync + Iterator<Item = B>,
	B: Bundle,
{
	pub value: T,
	_phantom: PhantomData<B>,
}

impl<T, B> BundleIter<T, B>
where
	T: 'static + Send + Sync + Iterator<Item = B>,
	B: Bundle,
{
	/// Create a new [`BundleIter`] effect.
	pub fn new(iter: T) -> Self {
		Self {
			value: iter,
			_phantom: PhantomData,
		}
	}
}

impl<T, B> BundleEffect for BundleIter<T, B>
where
	T: 'static + Send + Sync + Iterator<Item = B>,
	B: Bundle,
{
	fn apply(self, entity: &mut EntityWorldMut) {
		for bundle in self.value {
			entity.insert(bundle);
		}
	}
}
