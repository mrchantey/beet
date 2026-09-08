//! Dispatch-time reporting of a [`CfgExcluded`] tombstone: a route excluded by
//! `bx:cfg` still exists and still answers, naming the condition that removed
//! it instead of vanishing into "unknown route".

use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Observer: turn a route-shaped [`CfgExcluded`] tombstone into a route that
/// exists and refuses.
///
/// The tombstone records the excluded element's own `path`, so a
/// `<Route path="site" bx:cfg="..">` that was excluded still occupies `site` in
/// the tree: it is listed by `--help` and dispatching it reports the condition.
/// That is what lets `bx:cfg` be the single mechanism for exclusion rather than
/// needing a dispatch-time gate beside it.
///
/// A tombstone with no path (a `<Fragment>`, a plain element) gets no route,
/// because there is no name to answer at. It stays in the tree as data, which
/// is still enough for a diagnostic to find it.
///
/// The honest limit: routes BELOW the excluded node cannot be restored, since
/// the subtree that declared them never built. `site` reports the exclusion;
/// `site/deploy` is not found. Nothing could do better without running the
/// effect the exclusion existed to prevent.
pub(crate) fn report_cfg_excluded(
	ev: On<Add, CfgExcluded>,
	query: Query<&CfgExcluded>,
	mut commands: Commands,
) -> Result {
	let excluded = query.get(ev.entity)?.clone();
	if excluded.path.is_empty() {
		return Ok(());
	}
	let message = excluded.message();
	commands.entity(ev.entity).insert((
		PathPartial::new(excluded.path.as_str()),
		Action::<Request, Response>::new_pure(
			move |_cx: ActionContext<Request>| -> Result<Response> {
				bevybail!("{message}")
			},
		),
	));
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// The replacement for what a dispatch-time feature gate used to give:
	/// dispatching an excluded route names the condition rather than 404ing.
	#[beet_core::test]
	async fn dispatching_an_excluded_route_names_the_condition() {
		let mut world = router_world();
		world
			.spawn((Router::with_defaults(), children![CfgExcluded {
				condition: "feature:infra && feature:extra".into(),
				tag: "Route".into(),
				path: "site".into(),
			}]))
			.exchange(Request::get("site"))
			.await
			.into_result()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("excluded from this build")
			.xpect_contains("feature:infra && feature:extra");
	}

	/// A tombstone that recorded no path claims no route, so a sibling route of
	/// the same name is unaffected.
	#[beet_core::test]
	async fn a_pathless_tombstone_claims_nothing() {
		let mut world = router_world();
		world
			.spawn((Router::with_defaults(), children![
				CfgExcluded {
					condition: "feature:infra".into(),
					tag: "Fragment".into(),
					path: "".into(),
				},
				(
					PathPartial::new("run"),
					Action::<Request, Response>::new_pure(
						|_cx: ActionContext<Request>| Response::ok().xok()
					),
				)
			]))
			.exchange(Request::get("run"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}
}
