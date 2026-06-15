//! `ReactivityScript` widget: the browser half of the reactive wire format.

use beet_core::prelude::*;
use beet_net::prelude::*;

/// Injects the beet thin-client reactivity runtime as an inline `<script>`.
///
/// The runtime hydrates the page from the `data-bx-*` annotations and JSON blobs
/// the reactive [`HtmlRenderer`](beet_ui::prelude::HtmlRenderer) emits (see its
/// `render::reactive` wire-format contract) and drives `bx:<event>` verbs with no
/// WASM. It renders only while a long-running server is up ([`KeepAlive`]), which
/// is exactly when [`default_renderer`](crate::prelude::default_renderer) emits
/// the matching annotations, so a one-shot render and static export stay clean.
///
/// Registered by name (see [`RouterPlugin`](crate::prelude::RouterPlugin)), so a
/// BSX layout drops `<ReactivityScript/>` in its head beside `<LiveReloadScript/>`.
#[template(system)]
pub fn ReactivityScript(keep_alive: Option<Res<KeepAlive>>) -> Snippet {
	if keep_alive.is_none() {
		return Snippet::from_bundle(());
	}
	let runtime = include_str!("./reactivity.js");
	rsx! { <script>{runtime}</script> }.any_snippet()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	/// Render `<ReactivityScript/>` to an HTML string.
	fn render_html(world: &mut World) -> String {
		let root = world.spawn_template(rsx! { <ReactivityScript/> }).unwrap().id();
		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, &mut *world))
			.unwrap()
			.to_string()
	}

	#[beet_core::test]
	fn renders_the_runtime_while_serving() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(KeepAlive);
		render_html(&mut world)
			.xpect_contains("<script>")
			.xpect_contains("data-bx-blob")
			.xpect_contains("EntityMut");
	}

	#[beet_core::test]
	fn renders_nothing_for_a_one_shot_render() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		render_html(&mut world).xnot().xpect_contains("<script>");
	}
}
