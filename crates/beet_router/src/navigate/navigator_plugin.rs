use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "tui")]
use beet_net::prelude::*;


#[derive(Default)]
pub struct NavigatorPlugin;


impl Plugin for NavigatorPlugin {
	fn build(&self, app: &mut App) {
		// link click handling (internal nav vs external open) lives in
		// OpenLinkPlugin, which classifies a clicked `<a>` and routes it.
		app.init_plugin::<OpenLinkPlugin>()
			.add_observer(single_current_page);
		// keyboard history shortcuts (alt+left/right) ride the terminal input
		// layer. The message registration is idempotent and makes the shortcut
		// system validate even when no input plugin is composed in. `slide_nav`
		// steps between sibling slides, opt-in via a `SlideDeck` router marker so
		// the docs TUI keeps its plain-arrow page scroll.
		#[cfg(feature = "tui")]
		app.add_message::<bevy::input::keyboard::KeyboardInput>()
			.add_systems(Update, (nav_shortcuts, slide_nav));
		// the `TuiServer` registers its own `StartServer` / `StopServer`
		// observers in its `on_add` hook, so there is no central registry to
		// populate here; `beet_net` stays ignorant of the TUI.
	}
}

/// System: alt+left / alt+right drive the navigator back / forward, the browser's
/// history shortcuts.
///
/// The terminal bridge emits the Alt modifier as a bracketing `AltLeft` press
/// (mirroring how Shift+Tab arrives), so the modifier and the arrow land in the
/// same frame's key stream. The scroll input ignores alt+arrows, so plain arrows
/// still scroll.
#[cfg(feature = "tui")]
fn nav_shortcuts(
	mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
	navigators: Query<Entity, With<Navigator>>,
	mut commands: Commands,
) {
	use bevy::input::ButtonState;
	use bevy::input::keyboard::KeyCode;
	let (mut alt, mut back, mut forward) = (false, false, false);
	for key in keys.read().filter(|key| key.state == ButtonState::Pressed) {
		match key.key_code {
			KeyCode::AltLeft | KeyCode::AltRight => alt = true,
			KeyCode::ArrowLeft => back = true,
			KeyCode::ArrowRight => forward = true,
			_ => {}
		}
	}
	if !alt {
		return;
	}
	let Ok(navigator) = navigators.single() else {
		return;
	};
	if back {
		commands.entity(navigator).queue_async(Navigator::back);
	} else if forward {
		commands.entity(navigator).queue_async(Navigator::forward);
	}
}

/// System: arrow / space / page / home / end keys step the navigator between
/// sibling slides, opt-in via a [`SlideDeck`] marker on the in-world router.
///
/// Key map (no wrap, clamped at the ends):
/// - prev: Left, Up, PageUp
/// - next: Right, Down, PageDown, Space, Enter
/// - first: Home  |  last: End
///
/// Gated so it never hijacks the docs-TUI: only a navigator whose
/// [`NavigatorTransport::InWorld`] router carries [`SlideDeck`] navigates, and
/// Alt+arrows are left to [`nav_shortcuts`] (history), so a plain-arrow docs TUI
/// keeps scrolling.
#[cfg(feature = "tui")]
fn slide_nav(
	mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
	navigators: Query<(Entity, &Navigator)>,
	decks: Query<(), With<SlideDeck>>,
	route_trees: Query<&RouteTree>,
	mut commands: Commands,
) {
	use bevy::input::ButtonState;
	use bevy::input::keyboard::KeyCode;

	// classify this frame's keys into at most one slide step, skipping the
	// Alt-modified arrows that drive history in `nav_shortcuts`.
	let (mut alt, mut nav) = (false, None);
	for key in keys.read().filter(|key| key.state == ButtonState::Pressed) {
		match key.key_code {
			KeyCode::AltLeft | KeyCode::AltRight => alt = true,
			KeyCode::ArrowLeft | KeyCode::ArrowUp | KeyCode::PageUp => {
				nav = Some(SlideNav::Prev)
			}
			KeyCode::ArrowRight
			| KeyCode::ArrowDown
			| KeyCode::PageDown
			| KeyCode::Space
			| KeyCode::Enter => nav = Some(SlideNav::Next),
			KeyCode::Home => nav = Some(SlideNav::First),
			KeyCode::End => nav = Some(SlideNav::Last),
			_ => {}
		}
	}

	// every guard below is a clean no-op (never an error, so no log spam): a
	// non-slide key or Alt-arrow, no/HTTP navigator, an unmarked or not-yet-built
	// router (eg the docs TUI), or a resolution miss at a non-slide path.
	let Some((nav, (entity, navigator))) =
		nav.filter(|_| !alt).zip(navigators.single().ok())
	else {
		return;
	};
	let NavigatorTransport::InWorld { router } = navigator.transport() else {
		return;
	};
	let (true, Ok(tree)) = (decks.contains(*router), route_trees.get(*router))
	else {
		return;
	};
	let current_path = navigator.current_url().path().clone();
	let Ok(target) = resolve_slide(tree, &current_path, nav) else {
		return;
	};

	// navigate to the resolved slide's absolute path (clamped at the deck's ends).
	let url = Url::parse(format!("/{}", target.join("/")));
	commands
		.entity(entity)
		.queue_async(async move |entity| Navigator::navigate_to(entity, url).await);
}
