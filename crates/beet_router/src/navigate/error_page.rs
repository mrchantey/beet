//! The navigation-failure error page: a material [`#[template]`](ErrorPage)
//! shown in place of a page that failed to load.
//!
//! When a [`Navigator`](crate::prelude::Navigator) load fails (eg the network is
//! down on the initial home fetch) the failure is both logged (`error!`) and
//! surfaced to the user as a rendered page rather than left blank.
//! [`set_error_page`] builds the template and binds it to the failing navigator's
//! surface, so the live page host paints it through the same layout/`Portal` path
//! as any other page.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;

/// A user-facing error page: a material card with a heading and the error
/// message, styled like the rest of the UI.
///
/// Registered by name (see [`RouterPlugin`](crate::prelude::RouterPlugin)), so a
/// layout or markup site can place `<ErrorPage message=".."/>`. The document
/// chrome is the ancestor layout's job; this widget owns only the error card.
#[template]
pub fn ErrorPage(#[prop(into)] message: String) -> impl Bundle {
	rsx! {
		<div {Classes::new([classes::CARD_FILLED])}>
			<h1 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Page failed to load"</h1>
			<p {Classes::new([classes::ERROR_TEXT])}>{message}</p>
		</div>
	}
}

/// Build the [`ErrorPage`] for `message` and bind it to the failing navigator's
/// surface, so the live host paints it in place of the page that failed to load.
///
/// `host` is the navigator's [`PageHost`] surface. Built through `spawn_template`
/// (so its slots/lifecycle resolve) and marked [`DespawnAfterRender`] so it is
/// cleaned up when the next navigation replaces it, exactly like a parsed or
/// per-request page.
pub fn set_error_page(
	world: &mut World,
	host: Entity,
	message: impl Into<String>,
) {
	let message = message.into();
	let page = world
		.spawn_template(rsx! { <ErrorPage message=message/> })
		.map(|entity| entity.id());
	match page {
		Ok(page) => {
			world
				.entity_mut(page)
				.insert(DespawnAfterRender(vec![page]));
			bind_surface_page(world, host, page);
		}
		// a failure building the error page itself is logged; there is no further
		// fallback to render.
		Err(err) => error!("failed to build error page: {err}"),
	}
}

/// A loading placeholder: a centered card shown on a surface before its first
/// page resolves, so the live host paints "Loading…" rather than a blank screen
/// or a transient "no route" error while route discovery is still settling.
#[template]
pub fn LoadingPage() -> impl Bundle {
	rsx! {
		<div {Classes::new([classes::CARD_FILLED])}>
			<h1 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Loading…"</h1>
		</div>
	}
}

/// Build the [`LoadingPage`] and bind it to `host` (a surface's [`PageHost`]), so
/// the live host paints it until the first navigation replaces it. Mirrors
/// [`set_error_page`]: built through `spawn_template` and marked
/// [`DespawnAfterRender`] so the next page cleans it up.
pub fn set_loading_page(world: &mut World, host: Entity) {
	let page = world
		.spawn_template(rsx! { <LoadingPage/> })
		.map(|entity| entity.id());
	match page {
		Ok(page) => {
			world
				.entity_mut(page)
				.insert(DespawnAfterRender(vec![page]));
			bind_surface_page(world, host, page);
		}
		Err(err) => error!("failed to build loading page: {err}"),
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::math::UVec2;

	/// The live-TUI render stack minus the terminal host, matching `live_page`'s
	/// test app: charcell pipeline, per-frame repaint, document chain, page sync.
	fn live_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TemplatePlugin,
			DocumentPlugin,
			CharcellPlugin,
			RealtimeParsePlugin,
			LivePagePlugin,
		));
		app
	}

	/// The host buffer's painted frame as plain text after one frame.
	fn frame(app: &mut App, host: Entity) -> String {
		app.update();
		app.world()
			.get::<DoubleBuffer>(host)
			.unwrap()
			.current_buffer()
			.render_plain()
	}

	/// A failed load paints the error page into the live host: the heading and the
	/// error message both render.
	#[beet_core::test]
	fn error_page_paints_message() {
		let mut app = live_app();
		let host = app.world_mut().spawn(page_host(UVec2::new(60, 8))).id();
		set_error_page(app.world_mut(), host, "network down");
		frame(&mut app, host)
			.xpect_contains("Page failed to load")
			.xpect_contains("network down");
	}
}
