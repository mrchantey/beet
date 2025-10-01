use beet_core::prelude::*;



/// The vanilla asset event doesnt implement reflect so..
#[derive(Debug, Reflect, PartialEq, Event)]
pub enum ReflectedAssetEvent<A: Asset> {
	/// Emitted whenever an [`Asset`] is added.
	Added {
		/// The id of the asset that was added.
		id: AssetId<A>,
	},
	/// Emitted whenever an [`Asset`] value is modified.
	Modified {
		/// The id of the asset that was modified.
		id: AssetId<A>,
	},
	/// Emitted whenever an [`Asset`] is removed.
	Removed {
		/// The id of the asset that was removed.
		id: AssetId<A>,
	},
	/// Emitted when the last [`super::Handle::Strong`] of an [`Asset`] is dropped.
	Unused {
		/// The id of the asset that is now unused.
		id: AssetId<A>,
	},
	/// Emitted whenever an [`Asset`] has been fully loaded (including its dependencies and all "recursive dependencies").
	LoadedWithDependencies {
		/// The id of the asset that was loaded.
		id: AssetId<A>,
	},
}


impl<A: Asset> Into<AssetEvent<A>> for ReflectedAssetEvent<A> {
	fn into(self) -> AssetEvent<A> {
		match self {
			ReflectedAssetEvent::Added { id } => AssetEvent::Added { id },
			ReflectedAssetEvent::Modified { id } => AssetEvent::Modified { id },
			ReflectedAssetEvent::Removed { id } => AssetEvent::Removed { id },
			ReflectedAssetEvent::Unused { id } => AssetEvent::Unused { id },
			ReflectedAssetEvent::LoadedWithDependencies { id } => {
				AssetEvent::LoadedWithDependencies { id }
			}
		}
	}
}

impl<A: Asset> From<AssetEvent<A>> for ReflectedAssetEvent<A> {
	fn from(event: AssetEvent<A>) -> Self {
		match event {
			AssetEvent::Added { id } => ReflectedAssetEvent::Added { id },
			AssetEvent::Modified { id } => ReflectedAssetEvent::Modified { id },
			AssetEvent::Removed { id } => ReflectedAssetEvent::Removed { id },
			AssetEvent::Unused { id } => ReflectedAssetEvent::Unused { id },
			AssetEvent::LoadedWithDependencies { id } => {
				ReflectedAssetEvent::LoadedWithDependencies { id }
			}
		}
	}
}


impl<A: Asset> Clone for ReflectedAssetEvent<A> {
	fn clone(&self) -> Self { *self }
}

impl<A: Asset> Copy for ReflectedAssetEvent<A> {}
