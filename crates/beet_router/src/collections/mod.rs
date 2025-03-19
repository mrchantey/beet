#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod axum_router_collection;
// TODO get beet_router working with wasm
mod spa_route_collection;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod static_route_collection;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use axum_router_collection::*;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use static_route_collection::*;

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
