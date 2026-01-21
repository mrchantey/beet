//! Router exchange with static EndpointTree construction.
//!
//! This module provides [`router_exchange`], which wraps [`flow_exchange`] and
//! eagerly constructs the [`EndpointTree`] on spawn.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Creates a router exchange that constructs the [`EndpointTree`] immediately on spawn.
///
/// Unlike using [`flow_exchange`] directly, this function ensures that the endpoint tree
/// is built and validated statically when the router is spawned. 
/// This provides early detection of routing conflicts and ensures
/// the tree is available for all operations that need it.
///
/// ## Example
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # use beet_flow::prelude::*;
/// # use beet_router::prelude::*;
/// let mut world = World::new();
/// world.spawn(router_exchange(|| {
///     (InfallibleSequence, children![
///         EndpointBuilder::get().with_path("api"),
///         EndpointBuilder::get().with_path("users/:id"),
///     ])
/// }));
/// ```
pub fn router_exchange(func: impl BundleFunc) -> impl Bundle {
	let func2 = func.clone();
	(
		// insert EndpointTree using the BundleFunc on spawn
		OnSpawn::new(move |entity| {
			let id = entity.id();
			entity.world_scope(|world| {
				let endpoints = EndpointTree::endpoints_from_bundle_func(
					world,
					func2.clone(),
				)
				.unwrap_or_exit();
				let tree =
					EndpointTree::from_endpoints(endpoints).unwrap_or_exit();
				world.entity_mut(id).insert(tree);
			});
		}),
		flow_exchange(func),
	)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn endpoint_tree_inserted_on_spawn() {
		let mut world = RouterPlugin::world();
		let entity = world
			.spawn(router_exchange(|| {
				(InfallibleSequence, children![
					EndpointBuilder::get().with_path("foo"),
					EndpointBuilder::get().with_path("bar"),
				])
			}));

		// EndpointTree should be present immediately after spawn
		entity
			.get::<EndpointTree>()
			.is_some()
			.xpect_true();

		// Verify the tree has the expected endpoints
		let tree = entity.get::<EndpointTree>().unwrap();
		tree.flatten().len().xpect_eq(2);
	}

	#[beet_core::test]
	async fn exchange_works() {
		RouterPlugin::world()
			.spawn(router_exchange(|| {
				(InfallibleSequence, children![
					EndpointBuilder::get()
						.with_path("test")
						.with_action(|| "hello"),
					html_bundle_to_response(),
				])
			}))
			.exchange_str("/test")
			.await
			.xpect_eq("hello");
	}
}
