//! Bridges between the boot slot (`Action<Boot, Response>`) and the dispatch slot
//! (`Action<Request, Response>`), so a boot can drive the normal request pipeline
//! and one entry can boot another, without a bespoke server.
//!
//! [`BootToExchange`] makes a host's boot slot dispatch through its own request
//! pipeline. [`ExchangeToBoot`] makes a request handler boot another entity. Both
//! carry a [`GlobFilter`] selecting which of the boot/request params (the CliArgs)
//! propagate across the bridge; the rest stay behind. Gated on `action` like the
//! rest of the boot path, so an embedded boot bridges the same way.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Bridges a host's boot slot to its dispatch slot: an `Action<Boot, Response>` that
/// reinterprets the [`Boot`] as a [`Request`] (cheap, since `Boot` derefs to it) and
/// dispatches the host's own `Action<Request, Response>`. A boot then drives the
/// normal request pipeline with no bespoke server, eg
/// `<Router {(BootToExchange, BootOnLoad)}>` runs the booted path's command through
/// the route tree.
///
/// The [`filter`](Self::filter) selects which boot params (the CliArgs) propagate
/// into the dispatched request; the rest are boot-only. The default propagates all.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Action<Boot, Response> = Action::new_async_local(boot_to_exchange_action))]
pub struct BootToExchange {
	/// Which boot params carry into the dispatched request (the rest are boot-only).
	pub filter: GlobFilter,
}

/// The `Action<Boot, Response>` handler [`BootToExchange`] installs: reinterpret the
/// boot as a request (filtered) and dispatch the caller's own request slot.
async fn boot_to_exchange_action(cx: ActionContext<Boot>) -> Result<Response> {
	let caller = cx.caller.clone();
	let boot = cx.take();
	let filter = caller
		.with(|entity| {
			entity.get::<BootToExchange>().map(|bridge| bridge.filter.clone())
		})
		.await?
		.ok_or_else(|| {
			bevyhow!("BootToExchange action ran without its component")
		})?;
	Ok(caller.exchange(propagate_args(boot.0, &filter)).await)
}

/// Bridges a request handler to another entity's boot slot: an
/// `Action<Request, Response>` that propagates the incoming request into a [`Boot`]
/// call on [`target`](Self::target). The mechanism for one entry to boot another, eg
/// a workspace CLI's `serve` route booting the loaded site entry.
///
/// The [`filter`](Self::filter) selects which request params propagate into the
/// boot; the rest stay behind. The default propagates all.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Action<Request, Response> = Action::new_async_local(exchange_to_boot_action))]
pub struct ExchangeToBoot {
	/// The entity whose `Action<Boot, Response>` boot slot this drives.
	#[entities]
	pub target: Entity,
	/// Which request params carry into the boot (the rest stay behind).
	pub filter: GlobFilter,
}

impl Default for ExchangeToBoot {
	fn default() -> Self {
		Self {
			target: Entity::PLACEHOLDER,
			filter: GlobFilter::default(),
		}
	}
}

impl ExchangeToBoot {
	/// Boot `target` (propagating all request params) when this handler's route runs.
	pub fn new(target: Entity) -> Self {
		Self {
			target,
			..Default::default()
		}
	}
}

/// The `Action<Request, Response>` handler [`ExchangeToBoot`] installs: propagate the
/// request (filtered) into a boot call on the target entity.
async fn exchange_to_boot_action(
	cx: ActionContext<Request>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let request = cx.take();
	let (target, filter) = caller
		.with(|entity| {
			entity
				.get::<ExchangeToBoot>()
				.map(|bridge| (bridge.target, bridge.filter.clone()))
		})
		.await?
		.ok_or_else(|| {
			bevyhow!("ExchangeToBoot action ran without its component")
		})?;
	caller
		.world()
		.entity(target)
		.call::<Boot, Response>(Boot(propagate_args(request, &filter)))
		.await
}

/// Drop the params whose key the `filter` excludes (the boot-only CliArgs), keeping
/// the path and the included params: the request a bridge propagates across.
fn propagate_args(mut request: Request, filter: &GlobFilter) -> Request {
	let dropped = request
		.request_parts()
		.params()
		.keys()
		.filter(|key| !filter.passes(key.as_str()))
		.cloned()
		.collect::<Vec<_>>();
	let params = request.request_parts_mut().params_mut();
	for key in dropped {
		params.remove(&key);
	}
	request
}

#[cfg(test)]
mod test {
	use super::*;

	/// A handler echoing the request path + its `kept` param, to prove what a bridge
	/// propagated. Pairs with [`BootToExchange`] so the entity also has a boot slot.
	fn echo() -> impl Bundle {
		(
			exchange_handler(|cx| {
				let request = cx.take();
				let kept = request
					.get_param("kept")
					.map(|value| value.to_string())
					.unwrap_or_default();
				Response::ok().with_body(format!("{} kept={kept}", request.path_string()))
			}),
			BootToExchange::default(),
		)
	}

	/// The boot slot dispatches the host's own request pipeline: calling
	/// `Action<Boot, Response>` runs the request through `Action<Request, Response>`.
	#[beet_core::test]
	async fn boot_to_exchange_dispatches_request() {
		(MinimalPlugins, ServerPlugin)
			.into_world()
			.spawn(echo())
			.call::<Boot, Response>(Boot(Request::get("foo/bar")))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_eq("/foo/bar kept=");
	}

	/// The bridge filter drops the excluded params (`secret`) but keeps the path and
	/// the included params (`kept`).
	#[beet_core::test]
	async fn boot_to_exchange_filters_params() {
		(MinimalPlugins, ServerPlugin)
			.into_world()
			.spawn((
				exchange_handler(|cx| {
					let request = cx.take();
					let secret = request.get_param("secret").is_some();
					let kept = request.get_param("kept").is_some();
					Response::ok().with_body(format!("secret={secret} kept={kept}"))
				}),
				BootToExchange {
					filter: GlobFilter::default().with_exclude("secret"),
				},
			))
			.call::<Boot, Response>(Boot(
				Request::get("cmd").with_param("secret", "x").with_param("kept", "y"),
			))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_eq("secret=false kept=true");
	}

	/// `ExchangeToBoot` boots another entity: a request handler propagates into the
	/// target's boot slot, which dispatches the target's pipeline.
	#[beet_core::test]
	async fn exchange_to_boot_boots_target() {
		let mut world = (MinimalPlugins, ServerPlugin).into_world();
		let target = world.spawn(echo()).flush();
		world
			.spawn(ExchangeToBoot::new(target))
			.call::<Request, Response>(Request::get("baz"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_eq("/baz kept=");
	}
}
