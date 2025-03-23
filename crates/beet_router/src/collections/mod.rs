#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod axum_router_collection;
mod spa_route_collection;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use axum_router_collection::*;
pub use spa_route_collection::*;

use crate::prelude::*;


pub trait IntoCollection<M> {
	fn into_collection(self) -> impl Collection;
}

pub trait Collection {
	fn register(self, app: &mut AppRouter);
}


impl<F: FnOnce(&mut AppRouter)> Collection for F {
	fn register(self, app: &mut AppRouter) { self(app) }
}
