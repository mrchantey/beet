//! The `/health` endpoint, derived from live server state.
//!
//! A load balancer probes `/health` to know a task is live (the Fargate health
//! check path), and the same metrics (active sessions, uptime) drive autoscaling
//! decisions and human debugging. Both are read from the world directly (the app
//! clock and a query) rather than a hand-maintained resource.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
#[cfg(feature = "tui")]
use beet_ui::prelude::*;
use bevy::ecs::system::In;

/// A `GET /health` route returning 200 with a small json body (`status`,
/// `uptime_secs`, `active_sessions`), for load-balancer health checks and humans.
///
/// The orchestrator probes this to know the task is live; autoscaling tracks CPU
/// (and can also track `active_sessions` as a custom metric). Wired into the
/// default app routes, so every site gets it.
pub fn health_route() -> impl Bundle {
	(exchange_route("health", HealthHandler), HttpMethod::Get)
}

/// Derives the health metrics from live world state: `uptime_secs` from the app
/// clock ([`Time`]), `active_sessions` from the live remote terminal surfaces
/// ([`ChannelTerminal`](beet_ui::prelude::ChannelTerminal), one per SSH session,
/// which excludes the local stdio terminal). No bookkeeping resource to keep in
/// sync; without the `tui` feature no remote sessions exist, so the count is 0.
#[action(handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
fn HealthHandler(
	_cx: In<ActionContext<RequestParts>>,
	time: Option<Res<Time>>,
	#[cfg(feature = "tui")] sessions: Query<(), With<ChannelTerminal>>,
) -> MediaBytes {
	// `Time` is absent without a `TimePlugin` (eg a bare test world); report 0.
	let uptime = time.map(|time| time.elapsed().as_secs()).unwrap_or(0);
	#[cfg(feature = "tui")]
	let sessions = sessions.iter().count();
	#[cfg(not(feature = "tui"))]
	let sessions = 0usize;
	let body = format!(
		"{{\"status\":\"ok\",\"uptime_secs\":{uptime},\"active_sessions\":{sessions}}}"
	);
	MediaBytes::new(MediaType::Json, body.into_bytes())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn health_reports_ok() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(pkg_config!());
		world
			.spawn(default_router())
			.call::<Request, Response>(Request::get("health"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("\"status\":\"ok\"")
			.xpect_contains("active_sessions");
	}

	/// Active sessions count the live remote terminal surfaces; three
	/// [`ChannelTerminal`](beet_ui::prelude::ChannelTerminal)s stand in for three
	/// SSH sessions. Needs the `tui` feature (the charcell terminal lives there).
	#[cfg(feature = "tui")]
	#[beet_core::test]
	async fn health_counts_active_sessions() {
		use beet_ui::prelude::*;
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(pkg_config!());
		for _ in 0..3 {
			world.spawn(ChannelTerminal::new(TerminalConfig::default()).0);
		}
		world
			.spawn(default_router())
			.call::<Request, Response>(Request::get("health"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("\"active_sessions\":3");
	}
}
