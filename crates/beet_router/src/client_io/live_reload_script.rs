//! `LiveReloadScript` widget, the browser side of [`LiveReload`](super::LiveReload).

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Injects the live-reload client as an inline `<script>`: it connects to the
/// world's [`ClientIo`] channel, calls `location.reload()` on a
/// [`RELOAD_MESSAGE`](super::RELOAD_MESSAGE), and reconnects with exponential
/// backoff, reloading after the server restarts.
///
/// Registered by name (see [`RouterPlugin`](crate::prelude::RouterPlugin)), so
/// a BSX layout drops `<LiveReloadScript/>` in its head. Renders nothing when
/// no [`ClientIo`] channel is active (production, static export), so it is
/// always safe to include.
#[template(system)]
pub fn LiveReloadScript(channels: Query<&ClientIo>) -> Snippet {
	let Some(port) = channels
		.iter()
		.next()
		.map(|channel| channel.port.unwrap_or(DEFAULT_SOCKET_PORT))
	else {
		return Snippet::from_bundle(());
	};
	let body = format!(
		"const CLIENT_IO_PORT={port};\n{}",
		include_str!("./live_reload.js")
	);
	rsx! { <script>{body}</script> }.any_snippet()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_ui::prelude::*;

	/// Render a template to an HTML string through the substrate.
	fn render_html(world: &mut World) -> String {
		let root = world
			.spawn_template(rsx! { <LiveReloadScript/> })
			.unwrap()
			.id();
		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, &mut *world))
			.unwrap()
			.to_string()
	}

	#[beet_core::test]
	fn renders_the_script_against_the_channel() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.spawn(ClientIo { port: Some(7777) });
		render_html(&mut world)
			.xpect_contains("<script>")
			.xpect_contains("const CLIENT_IO_PORT=7777;")
			.xpect_contains("location.reload()");
	}

	#[beet_core::test]
	fn renders_nothing_without_a_channel() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		render_html(&mut world).xnot().xpect_contains("<script>");
	}
}
