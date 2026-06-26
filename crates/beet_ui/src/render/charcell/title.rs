//! Mirroring each surface's document `<title>` onto its terminal window title bar.
//!
//! The browser sets its tab title from the document `<title>`; the terminal has
//! the same affordance through an OSC-0 escape. [`collect_terminal_titles`]
//! resolves each `<title>` to the surface it belongs to and records its text as
//! a [`Title`] there; [`flush_terminal_titles`] writes a changed [`Title`] to its
//! own terminal. So the title bar follows navigation, fires only on real title
//! changes, and each surface (one per SSH session) shows only its own page's
//! title.
use super::*;
use crate::prelude::*;
use beet_core::prelude::*;

/// The window-title string currently shown on a surface's terminal title bar.
///
/// Written onto the surface entity (the one owning the [`Terminal`]) from its
/// page's `<title>` by [`collect_terminal_titles`], and consumed by
/// [`flush_terminal_titles`] when it changes — so the OSC-0 write fires on real
/// title changes, not every frame, and each surface gets only its own page's
/// title. The [`PartialEq`] derive is what makes the change-detection meaningful.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
pub(crate) struct Title(pub String);

/// System: resolve each page `<title>` to its surface and record the
/// (control-char-stripped) text as a [`Title`] on that surface.
///
/// Reads only `<title>` elements; for each, walks to its
/// [`RenderSurface`](crate::prelude::RenderSurface) via [`SurfaceQuery`] and
/// upserts the surface's [`Title`], but only when the string actually changes so
/// [`flush_terminal_titles`]' `Changed` gate stays meaningful. A `<title>`
/// outside any surface is skipped.
#[cfg(feature = "tui")]
pub(crate) fn collect_terminal_titles(
	elements: ElementQuery,
	surfaces: SurfaceQuery,
	titles: Query<&Title>,
	mut commands: Commands,
) {
	for view in elements.iter().filter(|view| view.tag() == "title") {
		let Some((_, value)) = view.inner_text else {
			continue;
		};
		let Ok(text) = value.as_str() else { continue };
		// OSC injection guard: strip control chars so the title cannot terminate
		// the escape sequence early.
		let text: String = text.chars().filter(|ch| !ch.is_control()).collect();
		let Some(surface) = surfaces.surface_of(view.entity) else {
			continue;
		};
		// write only on a real change, so PartialEq gates the Changed flag the
		// flush system reads.
		if titles
			.get(surface)
			.map(|title| title.0 != text)
			.unwrap_or(true)
		{
			commands.entity(surface).insert(Title(text));
		}
	}
}

/// System: write each surface's changed [`Title`] to its own terminal as an
/// OSC-0 set-window-title sequence (carried to stdout by [`flush_terminals`]).
///
/// `Changed`-gated, so an unchanged title never re-writes, and per-surface,
/// since it writes through the [`Terminal`] on the same entity the [`Title`]
/// sits on.
#[cfg(feature = "tui")]
pub(crate) fn flush_terminal_titles(
	mut changed: Populated<(&Title, &mut Terminal), Changed<Title>>,
) -> Result {
	for (title, mut terminal) in changed.iter_mut() {
		escape::set_window_title(terminal.writer_mut(), &title.0)?;
	}
	Ok(())
}

#[cfg(all(test, feature = "tui"))]
mod test {
	use super::*;

	/// The OSC-0 set-window-title sequence `escape::set_window_title` emits.
	fn osc_title(title: &str) -> String {
		format!("{}{title}{}", escape::OSC_WINDOW_TITLE, escape::BEL)
	}

	/// A booted charcell-render app driving [`PostParseTree`] each frame.
	fn app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, CharcellPlugin, RealtimeParsePlugin));
		app
	}

	/// Spawn a surface: a host entity owning a channel-backed [`Terminal`], plus a
	/// page tree carrying a `<title>` bound to that host via
	/// [`RenderSurface`](crate::prelude::RenderSurface). Returns the host entity.
	fn surface(app: &mut App, title: &str) -> Entity {
		let (channel, terminal) =
			ChannelTerminal::new(TerminalConfig::default());
		let host = app.world_mut().spawn((channel, terminal)).id();
		app.world_mut().spawn((RenderSurface(host), children![(
			Element::new("title"),
			children![Value::str(title)]
		)]));
		host
	}

	/// Drain a host terminal's emitted bytes since the last drain, as a string.
	fn drain(app: &mut App, host: Entity) -> String {
		app.world_mut()
			.get_mut::<ChannelTerminal>(host)
			.unwrap()
			.drain_write()
			.xmap(String::from_utf8)
			.unwrap()
	}

	/// Each terminal's title bar carries only its own surface's `<title>`: two
	/// surfaces with distinct titles never cross-write (the per-surface
	/// regression lock), and an unchanged title does not re-write.
	#[beet_core::test]
	fn titles_route_per_surface() {
		let mut app = app();
		let alpha = surface(&mut app, "Alpha");
		let beta = surface(&mut app, "Beta");
		app.update();

		// each terminal saw a set-title for its own title, and not the other's.
		let out_a = drain(&mut app, alpha);
		out_a
			.as_str()
			.xpect_contains(&osc_title("Alpha"))
			.xnot()
			.xpect_contains("Beta");
		let out_b = drain(&mut app, beta);
		out_b
			.as_str()
			.xpect_contains(&osc_title("Beta"))
			.xnot()
			.xpect_contains("Alpha");

		// an unchanged title does not re-write: the next frame emits no set-title.
		app.update();
		drain(&mut app, alpha)
			.xnot()
			.xpect_contains(&osc_title("Alpha"));
	}
}
