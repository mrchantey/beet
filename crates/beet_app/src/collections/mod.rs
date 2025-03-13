#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod axum_router_collection;
// TODO get beet_router working with wasm
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod file_route_collection;
mod spa_route_collection;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use axum_router_collection::*;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use file_route_collection::*;

use crate::prelude::*;


pub trait IntoCollection<M> {
	fn into_collection(self) -> impl Collection;
}

pub trait Collection {
	fn register(self, app: &mut BeetApp);
}


impl<F: FnOnce(&mut BeetApp)> Collection for F {
	fn register(self, app: &mut BeetApp) { self(app) }
}
