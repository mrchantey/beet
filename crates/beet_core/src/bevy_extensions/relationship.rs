use crate::prelude::*;


#[extend::ext]
pub impl<T> T
where
	T: RelationshipTarget,
	T::Collection: Clone,
{
	fn to_vec(&self) -> Vec<Entity> { self.iter().collect() }
}
