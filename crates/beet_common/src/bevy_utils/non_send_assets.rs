use bevy::ecs::component::ComponentHook;
use bevy::ecs::component::Immutable;
use bevy::ecs::component::StorageType;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;




/// A non-send resource similar to [`Assets`] but for non-send types.
#[derive(Debug, Clone, Deref)]
pub struct NonSendAssets<T>(HashMap<NonSendHandle<T>, T>);


impl<T> NonSendAssets<T> {
	pub fn insert(&mut self, value: T) -> NonSendHandle<T> {
		let handle = NonSendHandle::new(self.0.len());
		self.0.insert(handle, value);
		handle
	}

	pub fn get(&self, handle: &NonSendHandle<T>) -> anyhow::Result<&T> {
		self.0
			.get(handle)
			.ok_or_else(|| anyhow::anyhow!("Handle not found: {:#?}", handle))
	}
	pub fn get_mut(
		&mut self,
		handle: &NonSendHandle<T>,
	) -> anyhow::Result<&mut T> {
		self.0
			.get_mut(handle)
			.ok_or_else(|| anyhow::anyhow!("Handle not found: {:#?}", handle))
	}

	pub fn remove(&mut self, handle: &NonSendHandle<T>) -> anyhow::Result<T> {
		self.0
			.remove(handle)
			.ok_or_else(|| anyhow::anyhow!("Handle not found: {:#?}", handle))
	}
	pub fn into_inner(self) -> HashMap<NonSendHandle<T>, T> { self.0 }
}

impl<T> Default for NonSendAssets<T> {
	fn default() -> Self { Self(HashMap::default()) }
}

/// A handle to a [`NonSendAssets`] item.
/// These handles are not smart, removing an item from the [`NonSendAssets`]
/// must be done manually.
#[derive(Reflect)]
// #[component(on_add=foo)]
// #[component(on_remove=bar)]
pub struct NonSendHandle<T> {
	/// The hashmap key
	key: usize,
	phantom: std::marker::PhantomData<T>,
}

impl<T> std::fmt::Debug for NonSendHandle<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("NonSendHandle")
			.field("key", &self.key)
			.field("phantom", &std::any::type_name::<T>())
			.finish()
	}
}


impl<T: 'static> Component for NonSendHandle<T> {
	const STORAGE_TYPE: StorageType = StorageType::Table;
	type Mutability = Immutable;

	fn on_remove() -> Option<ComponentHook> {
		Some(|mut world, cx| {
			let handle = *world.get::<NonSendHandle<T>>(cx.entity).unwrap();
			world
				.non_send_resource_mut::<NonSendAssets<T>>()
				.remove(&handle)
				// don't panic, it may be removed already
				.ok();
		})
	}
}


impl<T> NonSendHandle<T> {
	fn new(key: usize) -> Self {
		Self {
			key,
			phantom: Default::default(),
		}
	}
	pub fn inner(&self) -> usize { self.key }
}

impl<T> Clone for NonSendHandle<T> {
	fn clone(&self) -> Self {
		Self {
			key: self.key,
			phantom: Default::default(),
		}
	}
}

impl<T> PartialEq for NonSendHandle<T> {
	fn eq(&self, other: &Self) -> bool { self.key == other.key }
}

impl<T> std::hash::Hash for NonSendHandle<T> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.key.hash(state);
	}
}

impl<T> Copy for NonSendHandle<T> {}
impl<T> Eq for NonSendHandle<T> {}
// SAFETY: T is only used for phantom data
unsafe impl<T> Send for NonSendHandle<T> {}
// SAFETY: T is only used for phantom data
unsafe impl<T> Sync for NonSendHandle<T> {}

#[extend::ext(name=EntityWorldMutNonSendExt)]
pub impl<'a> EntityWorldMut<'a> {
	fn insert_non_send<T: 'static>(&mut self, value: T) -> &mut Self {
		let id = self.id();
		self.world_scope(|world| {
			world.init_non_send_resource::<NonSendAssets<T>>();
			let mut assets = world.non_send_resource_mut::<NonSendAssets<T>>();
			let handle = assets.insert(value);
			world.entity_mut(id).insert(handle);
		});
		self
	}
	fn remove_non_send<T: 'static>(&mut self) -> Result<Option<T>> {
		let Some(handle) = self.get::<NonSendHandle<T>>().map(|h| *h) else {
			return Ok(None);
		};
		self.world_scope(|world| {
			let mut assets = world.non_send_resource_mut::<NonSendAssets<T>>();
			let value = assets.remove(&handle)?;
			Ok(Some(value))
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		// let assets = ::default();
		App::new()
			.init_non_send_resource::<NonSendAssets<u32>>()
			.add_systems(
				Startup,
				|mut assets: NonSendMut<NonSendAssets<u32>>| {
					let handle = assets.insert(8);
					expect(handle.inner()).to_be(0);
				},
			)
			.run();
	}
}
