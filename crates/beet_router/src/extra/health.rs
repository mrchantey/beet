//! The `/health` endpoint and the live [`ServerMetrics`] it reports.
//!
//! A load balancer probes `/health` to know a task is live (the Fargate health
//! check path), and the same metrics (active sessions, uptime) drive autoscaling
//! decisions and human debugging.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::system::In;

/// Live server metrics, surfaced at `/health` and usable as an autoscaling signal.
///
/// `active_sessions` is maintained by the SSH-TUI server (incremented on connect,
/// decremented on disconnect); `uptime` is measured from when the resource was
/// created (server start).
#[derive(Debug, Resource)]
pub struct ServerMetrics {
	/// Currently connected interactive sessions (eg SSH TUIs).
	pub active_sessions: u32,
	/// When the server started, for uptime reporting.
	started: Instant,
}

impl Default for ServerMetrics {
	fn default() -> Self {
		Self {
			active_sessions: 0,
			started: Instant::now(),
		}
	}
}

impl ServerMetrics {
	/// Seconds since the server started.
	pub fn uptime_secs(&self) -> u64 { self.started.elapsed().as_secs() }
}

/// A `GET /health` route returning 200 with a small json body (`status`,
/// `uptime_secs`, `active_sessions`), for load-balancer health checks and humans.
///
/// The orchestrator probes this to know the task is live; autoscaling tracks CPU
/// (and can also track `active_sessions` as a custom metric). Wired into the
/// default app routes, so every site gets it.
pub fn health_route() -> impl Bundle {
	(exchange_route("health", HealthHandler), HttpMethod::Get)
}

#[action(handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
fn HealthHandler(
	_cx: In<ActionContext<RequestParts>>,
	metrics: Option<Res<ServerMetrics>>,
) -> MediaBytes {
	let (uptime, sessions) = metrics
		.map(|metrics| (metrics.uptime_secs(), metrics.active_sessions))
		.unwrap_or((0, 0));
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

	#[beet_core::test]
	async fn health_reports_active_sessions() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(pkg_config!());
		world.insert_resource(ServerMetrics {
			active_sessions: 3,
			..default()
		});
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
