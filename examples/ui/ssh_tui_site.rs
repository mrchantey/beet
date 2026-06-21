//! Serve a small navigable site over HTTP and SSH at once: html over http, an
//! independent live terminal per ssh connection, from one process.
//!
//! The multi-tenant demo and the deployable entrypoint pattern (a dedicated
//! binary that spawns the router + servers, like `hello_fargate`).
//!
//! ```sh
//! cargo run --example ssh_tui_site --features ssh_tui,http_server,markdown
//! # html over http:
//! curl localhost:8337
//! curl localhost:8337/health
//! # a live TUI over ssh (open many at once, each independent):
//! ssh -p 8322 guest@localhost -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null
//! ```
//!
//! In the terminal: arrow keys scroll, links navigate, Tab moves focus, alt+left
//! goes back, ctrl+c disconnects that session (the server keeps serving others).

use beet::prelude::*;
use bevy::app::ScheduleRunnerPlugin;
use core::time::Duration;

fn main() -> Result {
	let mut app = App::new();
	app.add_plugins((
		// cap the repaint loop at ~30fps rather than busy-looping: a TUI needs no
		// more, and a busy loop would peg the cpu, breaking cpu-based autoscaling
		// (every task would read as maxed regardless of load).
		MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
			Duration::from_secs_f64(1.0 / 30.0),
		)),
		LogPlugin::default(),
		// the router + server backends + template/charcell pipeline.
		RouterPlugin,
		// the live interactive terminal stack (input bridge, repaint, navigation).
		CharcellTuiPlugin,
		NavigatorPlugin,
		LivePagePlugin,
		// the multi-tenant SSH-TUI per-connection behavior.
		SshTuiPlugin,
		// the style rule set the pages render with.
		material::MaterialStylePlugin::default(),
	))
	.insert_resource(pkg_config!());

	// one router carrying both servers + the demo routes; the boot fan-out boots
	// every server present (http + ssh), each on its own async accept loop, while
	// the bevy loop repaints every ssh surface each frame.
	app.world_mut()
		.spawn((
			default_router(),
			// `default()` reads `BEET_PORT` / `BEET_HOST` (a deployed task sets
			// `BEET_HOST=0.0.0.0`), falling back to localhost:8337 locally. The
			// SshTuiServer reads `BEET_SSH_PORT` / `BEET_HOST` the same way.
			HttpServer::default(),
			SshTuiServer,
			children![
				render_action::func_route("", |_: ()| home()),
				render_action::func_route("about", |_: ()| about()),
			],
		))
		.trigger(StartRunning::boot);

	app.run();
	Ok(())
}

/// The home page, shared by the http (html) and ssh (terminal) renders.
fn home() -> impl Bundle {
	rsx! {
		<div>
			<h1>"Beet over SSH"</h1>
			<p>"This page is served as html over http and as a live terminal over ssh, from one process."</p>
			<ul>
				<li><a href="/about">"About this demo"</a></li>
			</ul>
			<p>"Arrow keys scroll, Tab moves focus, alt+left goes back, ctrl+c disconnects."</p>
		</div>
	}
}

/// The about page.
fn about() -> impl Bundle {
	rsx! {
		<div>
			<h1>"About"</h1>
			<p>"Every ssh connection gets its own independent terminal: its own page, scroll position, focus and navigation history. Many can connect at once."</p>
			<a href="/">"Back home"</a>
		</div>
	}
}
