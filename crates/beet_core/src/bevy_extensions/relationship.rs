use crate::prelude::*;


/// Extension trait for collecting [`RelationshipTarget`] entities into a [`Vec`].
#[extend::ext(name=RelationshipTargetExt)]
pub impl<T> T
where
	T: RelationshipTarget,
	T::Collection: Clone,
{
	/// Collect all related entities into a [`Vec`].
	fn to_vec(&self) -> Vec<Entity> { self.iter().collect() }
}
