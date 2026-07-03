use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A template route at `/app-info` rendering the [`PackageConfig`] as an article.
///
/// Requires a [`PackageConfig`] resource (eg via `pkg_config!()`).
pub fn app_info() -> impl Bundle {
	(
		render_action::func_route(
			"app-info",
			|_: ()| rsx! { <AppInfoContent/> },
		),
		// live diagnostics (uptime, request count) must never be cached
		CacheHeaders::no_store(),
	)
}

/// Reads live diagnostics at template build into an `<article>`: the
/// [`PackageConfig`] fields, process uptime (Bevy's [`Time::elapsed`], which tracks
/// time since app start), and the running request count ([`ExchangeStats`], summed
/// across server entities).
///
/// `Time`/`ExchangeStats` are read optionally, so an app without them (eg a bare
/// test world without Bevy's `TimePlugin`) still renders the package fields.
#[template(system)]
fn AppInfoContent(
	config: Res<PackageConfig>,
	time: Option<Res<Time>>,
	servers: Query<&ExchangeStats>,
) -> impl Bundle {
	let PackageConfig {
		title,
		description,
		version,
		stage,
		..
	} = config.clone();
	// uptime since app start; "unknown" without a `Time` resource (no `TimePlugin`,
	// eg a bare router test world).
	let uptime = time
		.map(|time| time_ext::pretty_print_duration(time.elapsed()))
		.unwrap_or_else(|| "unknown".to_string());
	// `ExchangeStats` is a per-server component (required by `HttpServer`), so sum
	// across every server entity; 0 when nothing has served yet (eg the Worker
	// serves each request directly, without a booted server).
	let requests: u128 = servers.iter().map(ExchangeStats::request_count).sum();
	rsx! {
		<article>
			<h1>"App Info"</h1>
			<p>"Title: "{title}</p>
			<p>"Description: "{description}</p>
			<p>"Version: "{version}</p>
			<p>"Stage: "{stage}</p>
			<p>"Uptime: "{uptime}</p>
			<p>"Requests: "{requests.to_string()}</p>
		</article>
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use bevy::time::TimePlugin;

	#[beet_core::test]
	async fn renders_package_title() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(pkg_config!());
		// `default_router` already wires `app_info()` as a child under std.
		world
			.spawn(default_router())
			.exchange(Request::get("app-info"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("App Info")
			.xpect_contains("beet_router");
	}

	/// With Bevy's `TimePlugin` providing [`Time`] and a server that has already
	/// served two requests, `/app-info` reports uptime and the summed request count
	/// beside the package fields. A freshly built world reports `Uptime: 0`, so the
	/// assertion checks the label is present, not a specific value.
	#[beet_core::test]
	async fn reports_uptime_and_requests() {
		let mut world = (AsyncPlugin, RouterPlugin, TimePlugin).into_world();
		world.insert_resource(pkg_config!());
		// a server entity that has already served two requests, summed into the count.
		world.spawn(ExchangeStats::new(2));
		world
			.spawn(default_router())
			.exchange(Request::get("app-info"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("App Info")
			.xpect_contains("beet_router")
			.xpect_contains("Uptime: ")
			.xpect_contains("Requests: 2");
	}
}
