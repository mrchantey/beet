//! Dispatch-time enforcement of [`RequireFeatures`]: a route at or under the
//! declaration fails a call with the missing feature list instead of running
//! whichever of its steps this binary happened to link.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Observer: enforce an inserted [`RequireFeatures`] at dispatch time.
///
/// Pushes a middleware onto the declaring entity's [`MiddlewareList`], so any
/// route dispatched at or beneath it checks the compiled feature set before
/// its action runs: met calls through, unmet fails naming every missing
/// feature and the rebuild that fixes it. This is what keeps a lean binary's
/// inert deploy subtree loud without any runtime layer knowing how it loaded:
/// the requirement is a plain component, the check plain dispatch.
pub(crate) fn enforce_require_features(
	ev: On<Add, RequireFeatures>,
	query: Query<&RequireFeatures>,
	mut commands: Commands,
) -> Result {
	let features = query.get(ev.entity)?.clone();
	commands
		.entity(ev.entity)
		.queue(move |mut entity: EntityWorldMut| {
			entity
				.get_mut_or_default::<MiddlewareList<Request, Response>>()
				.add(move |request: Request, next: Next<Request, Response>| {
					let features = features.clone();
					async move {
						let failures = next
							.world()
							.with_state::<Query<&CrateRegistration>, _>(
								move |registrations| {
									features.failures(&registrations)
								},
							)
							.await;
						if failures.is_empty() {
							return next.call(request).await;
						}
						bevybail!(
							"this route requires features this binary was not \
							compiled with:\n  {}\nrebuild with the missing \
							features, ie `cargo install --path crates/beet-cli \
							--all-features`",
							failures.join("\n  ")
						)
					}
				});
		});
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// An unmet requirement fails dispatch before any step runs, naming the
	/// missing features rather than the symptom: the actionable error for a
	/// binary that loaded the subtree inert.
	#[beet_core::test]
	async fn unmet_features_fail_dispatch() {
		let mut world = router_world();
		world.spawn(
			CrateRegistration::new("beet-cli", "0.0.9").with_skip_prefix(),
		);
		world
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("deploy"),
				RequireFeatures::new("infra,extra"),
				ExchangeSequence,
				children![UnregisteredTag::new("TofuApply")],
			)]))
			.exchange(Request::get("deploy"))
			.await
			.into_result()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("beet-cli/infra (feature not compiled in)")
			.xpect_contains("rebuild with the missing features");
	}

	/// A met requirement is a pass-through.
	#[beet_core::test]
	async fn met_features_call_through() {
		let mut world = router_world();
		world.spawn(
			CrateRegistration::new("beet-cli", "0.0.9")
				.with_feature("infra")
				.with_feature("extra")
				.with_skip_prefix(),
		);
		world
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("run"),
				RequireFeatures::new("infra,extra"),
				ExchangeSequence,
				children![
					Action::<Request, Outcome<Request, Response>>::new_pure(
						|cx: ActionContext<Request>| Outcome::Pass(cx.take())
							.xok()
					)
				],
			)]))
			.exchange(Request::get("run"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	/// The requirement guards a whole subtree from an ancestor: dispatching a
	/// leaf route under a declaring parent takes the same failure.
	#[beet_core::test]
	async fn ancestor_requirement_guards_leaf_routes() {
		let mut world = router_world();
		world.spawn(
			CrateRegistration::new("beet-cli", "0.0.9").with_skip_prefix(),
		);
		world
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("shared"),
				RequireFeatures::new("infra,extra"),
				children![(
					PathPartial::new("push"),
					ExchangeSequence,
					children![UnregisteredTag::new("DirSync")],
				)],
			)]))
			.exchange(Request::get("shared/push"))
			.await
			.into_result()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("beet-cli/infra (feature not compiled in)");
	}
}
