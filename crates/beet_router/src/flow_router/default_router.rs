use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;



/// Create a router with a [`RunOnReady`] step, allowing any
/// [`ReadyAction`] children to complete before inserting the
/// [`RouteServer`] which will immediately start handling requests.
pub fn serve_on_ready() -> impl Bundle {
	(
		ReadyOnChildrenReady::default(),
		OnSpawn::observe(|ev: On<Ready>, mut commands: Commands| {
			if ev.event_target() == ev.original_event_target() {
				commands.entity(ev.event_target()).insert(RouteServer);
			}
		}),
	)
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	#[rustfmt::skip]
	async fn works() {
		FlowRouterPlugin::world()
			.spawn((
				serve_on_ready(),
				EndpointBuilder::get(),
				children![(
					EndWith(Outcome::Pass),
					ReadyAction::new(async |_| {})
				)],
			))
			.await_ready()
			.await
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}
}
