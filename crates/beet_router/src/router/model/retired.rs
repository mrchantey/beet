//! Retiring a route: hidden from dispatch at once, despawned once no request
//! holds it, so a swap that replaces a route never cuts a call to it short.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Marks a route a swap has replaced: it leaves the [`RouteTree`] at once (the
/// required [`RouteHidden`] wakes the rebuild) and despawns once its
/// [`RouteInFlight`] count reaches zero, through [`despawn_idle_retired`].
///
/// Retire through [`Retired::retire`], which despawns an idle route outright
/// rather than marking it.
#[derive(Debug, Default, Component)]
#[require(RouteHidden)]
pub(crate) struct Retired;

impl Retired {
	/// Retire `route`: despawned at once if no request holds it, else marked
	/// [`Retired`] so it leaves dispatch now and despawns once the last
	/// request answers.
	pub(crate) fn retire(world: &mut World, route: Entity) {
		let idle = world
			.get::<RouteInFlight>(route)
			.is_none_or(RouteInFlight::is_idle);
		match idle {
			true => world.entity_mut(route).despawn(),
			false => {
				world.entity_mut(route).insert(Retired);
			}
		}
	}
}

/// Despawn every [`Retired`] route no request holds any more, each tick
/// while any remains.
pub(crate) fn despawn_idle_retired(
	retired: Populated<(Entity, Option<&RouteInFlight>), With<Retired>>,
	mut commands: Commands,
) {
	for (route, _) in retired
		.iter()
		.filter(|(_, in_flight)| in_flight.is_none_or(RouteInFlight::is_idle))
	{
		commands.entity(route).despawn();
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn ok_route(path: &str) -> impl Bundle {
		route::new(path, exchange_ext::handler(|_| Response::ok()))
	}

	/// An idle route retires by despawning; a held one leaves the tree at once
	/// but stays alive for the request holding it, and the sweep despawns it
	/// only once that request has released it.
	#[beet_core::test]
	fn held_route_outlives_its_retirement() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let router = world
			.spawn((Router, children![
				ok_route("kept"),
				ok_route("idle"),
				ok_route("held")
			]))
			.flush();
		let tree = RouteTree::of(&world, router).unwrap();
		let idle = tree.find(&["idle"]).unwrap().entity;
		let held = tree.find(&["held"]).unwrap().entity;
		let guard = world.get::<RouteInFlight>(held).unwrap().begin();

		Retired::retire(&mut world, idle);
		Retired::retire(&mut world, held);
		world.flush();
		world.get_entity(idle).is_err().xpect_true();
		world.entity(held).contains::<RouteHidden>().xpect_true();
		let tree = RouteTree::of(&world, router).unwrap();
		tree.find(&["held"]).xpect_none();
		tree.find(&["kept"]).xpect_some();

		// still held: the sweep leaves it
		world.run_system_cached(despawn_idle_retired).unwrap();
		world.get_entity(held).is_ok().xpect_true();
		// released: the sweep despawns it
		drop(guard);
		world.run_system_cached(despawn_idle_retired).unwrap();
		world.get_entity(held).is_err().xpect_true();
	}
}
