use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use extend::ext;


// trait IntoWorldRef {
// 	fn into_world_ref(&self) -> &World;
// }
// impl IntoWorldRef for World {
// 	fn into_world_ref(&self) -> &World { self }
// }
// impl IntoWorldRef for App {
// 	fn into_world_ref(&self) -> &World { self.world() }
// }
trait IntoWorldMut {
	fn into_world_mut(&mut self) -> &mut World;
}
impl IntoWorldMut for World {
	fn into_world_mut(&mut self) -> &mut World { self }
}
impl IntoWorldMut for App {
	fn into_world_mut(&mut self) -> &mut World { self.world_mut() }
}

#[ext(name=WorldMutExt)]
/// Matcher extensions for `bevy::World`
pub impl<W: IntoWorldMut> W {
	/// Shorthand for creating a query and immediatly collecting it into a Vec.
	/// This is less efficient than caching the [`QueryState`] so should only be
	/// used for one-off queries, otherwise `world.query::<D>()` should be preferred.
	fn query_once<'a, D: QueryData>(&'a mut self) -> Vec<D::Item<'a>> {
		let world = self.into_world_mut();
		world.query::<D>().iter_mut(world).collect::<Vec<_>>()
	}
}
