use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;

/// Signifies this position in the route graph is a canonical handler,
/// and should be included in any kind of 'collect all endpoints' functionality.
///
/// Usually this is not added directly, instead via the [`endpoint`] constructor.
/// Endpoints should only run if there are no trailing path segments,
/// unlike middleware which may run for multiple child paths. See [`check_exact_path`]
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = on_add)]
pub struct Endpoint;


impl Endpoint {}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	let entity = cx.entity;
	world.commands().queue(move |world: &mut World| {
		let route_segments = world
			.run_system_cached_with(RouteSegments::collect, entity)
			.unwrap();
		world
			.entity_mut(entity)
			.insert(EndpointMeta::new(route_segments));
	});
}

/// Endpoints are actions that will only run if the method and path are an
/// exact match.
/// - A [`RouteSegments`] will be added, collecting all parent [`PathFilter`]s
/// - The method will also be added to the handler for easier querying.
pub fn endpoint<M>(
	method: HttpMethod,
	handler: impl IntoEndpoint<M>,
) -> impl Bundle {
	(Sequence, children![
		check_exact_path(),
		check_method(method),
		(Endpoint, method, handler.into_endpoint())
	])
}


/// An [`endpoint`] with a preceding path filter.
pub fn endpoint_with_path<M>(
	path: PathFilter,
	method: HttpMethod,
	handler: impl IntoEndpoint<M>,
) -> impl Bundle {
	// path filter must be ancestor of endpoint
	// so we nest the sequence
	parse_path_filter(path, endpoint(method, handler))
}



fn check_exact_path() -> impl Bundle {
	OnSpawn::observe(
		|mut ev: On<GetOutcome>, mut query: RouteQuery| -> Result {
			let outcome =
				query.with_cx(&mut ev, |cx| match cx.path().is_empty() {
					true => Outcome::Pass,
					false => Outcome::Fail,
				})?;
			// println!("check_exact_path: {}", outcome);
			ev.trigger_next(outcome);
			Ok(())
		},
	)
}

fn check_method(method: HttpMethod) -> impl Bundle {
	(
		method,
		OnSpawn::observe(
			|mut ev: On<GetOutcome>,
			 query: RouteQuery,
			 actions: Query<&HttpMethod>|
			 -> Result {
				let method = actions.get(ev.action())?;
				let outcome = match query.method(&ev)? == *method {
					true => Outcome::Pass,
					false => Outcome::Fail,
				};
				// println!("check_method: {}", outcome);
				ev.trigger_next(outcome);

				Ok(())
			},
		),
	)
}


/// Parses the [`RouteContext`] for this entity and applies the
/// [`PathFilter`], popping from the [`RouteContext::path`]
/// and inserting to the [`RouteContext::dyn_segments`].
/// The child will only run if the path matches, extra segments
/// are allowed.
pub fn parse_path_filter(
	filter: PathFilter,
	child: impl Bundle,
) -> impl Bundle {
	(
		filter,
		OnSpawn::observe(
			|mut ev: On<GetOutcome>,
			 mut query: RouteQuery,
			 actions: Query<&PathFilter>,
			 children: Query<&Children>|
			 -> Result {
				let filter = actions.get(ev.action())?;
				let outcome =
					query.with_cx(&mut ev, |cx| cx.parse_filter(filter))?;
				match outcome {
					Ok(_) => {
						let child = children.get(ev.action())?[0];
						ev.trigger_next_with(child, GetOutcome);
					}
					Err(_) => {
						ev.trigger_next(Outcome::Fail);
					}
				}

				// println!("check_path_filter: {}", outcome);
				Ok(())
			},
		),
		children![child],
	)
}


pub fn handler<Endpoint, M>(handler: impl IntoEndpoint<M>) -> impl Bundle {
	handler.into_endpoint()
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut world = FlowRouterPlugin::world();
		let mut entity = world.spawn((
			RouteServer,
			endpoint_with_path(
				PathFilter::new("foo"),
				HttpMethod::Post,
				StatusCode::OK,
			),
		));

		// method and path match
		entity
			.oneshot(Request::post("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
		// method does not match
		entity
			.oneshot(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
		// path does not match
		entity
			.oneshot(Request::get("/bar"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
		// path has extra parts
		entity
			.oneshot(Request::get("/foo/bar"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}
}
