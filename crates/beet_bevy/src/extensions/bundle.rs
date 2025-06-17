use crate::prelude::OnSpawnBoxed;
use bevy::prelude::*;


pub trait BundleExt {
	/// Converts the bundle into a boxed `OnSpawn`, allowing branches
	/// to return the same type.
	///
	/// ## Example
	/// # use beet_bevy::prelude::*;
	///
	/// let bundle = if true{
	/// 	().any_bundle()
	/// }else{
	/// 	Name::new("foo").any_bundle()
	/// }
	///
	fn any_bundle(self) -> OnSpawnBoxed;
}

impl<B: 'static + Send + Sync + Bundle> BundleExt for B {
	fn any_bundle(self) -> OnSpawnBoxed {
		OnSpawnBoxed::new(|entity| {
			entity.insert(self);
		})
	}
}
