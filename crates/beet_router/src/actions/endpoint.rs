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
pub struct Endpoint;


/// Endpoints are actions that will only run if the method and path are an
/// exact match.
/// - A [`RouteSegments`] will be added, collecting all parent [`PathFilter`]s
/// - The method will also be added to the handler for easier querying.
pub fn endpoint(method: HttpMethod, handler: impl Bundle) -> impl Bundle {
	(Sequence, children![
		check_exact_path(),
		check_method(method),
		(Endpoint, collect_route_segments(), method, handler,)
	])
}


/// An [`endpoint`] with a preceding path filter.
pub fn endpoint_with_path(
	path: PathFilter,
	method: HttpMethod,
	handler: impl Bundle,
) -> impl Bundle {
	// path filter must be ancestor of endpoint
	// so we nest the sequence
	(Sequence, children![(Sequence, children![(
		parse_path_filter(path),
		endpoint(method, handler)
	)])])
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
/// and inserting to the [`RouteContext::dyn_segments`]
pub fn parse_path_filter(filter: PathFilter) -> impl Bundle {
	(
		filter,
		OnSpawn::observe(
			|mut ev: On<GetOutcome>,
			 mut query: RouteQuery,
			 actions: Query<&PathFilter>|
			 -> Result {
				let filter = actions.get(ev.action())?;
				let outcome = query.with_cx(&mut ev, |cx| {
					match cx.parse_filter(filter) {
						Ok(_) => Outcome::Pass,
						Err(_) => Outcome::Fail,
					}
				})?;
				// println!("check_path_filter: {}", outcome);
				ev.trigger_next(outcome);
				Ok(())
			},
		),
	)
}

pub fn respond_with(
	response: impl 'static + Send + Sync + Clone + IntoResponse,
) -> impl Bundle {
	OnSpawn::observe(move |mut ev: On<GetOutcome>, mut commands: Commands| {
		let response = response.clone().into_response();
		commands.entity(ev.agent()).insert(response);
		ev.trigger_next(Outcome::Pass);
	})
}


pub fn handler<F>(
	response: impl 'static + Send + Sync + Clone + IntoResponse,
) -> impl Bundle {
	OnSpawn::observe(move |mut ev: On<GetOutcome>, mut commands: Commands| {
		let response = response.clone().into_response();
		commands.entity(ev.agent()).insert(response);
		ev.trigger_next(Outcome::Pass);
	})
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
				respond_with(StatusCode::OK),
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
