use crate::prelude::*;
use beet_core::prelude::*;


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
		// system validate even when no input plugin is composed in. Stepping
		// between sibling cards (`card_nav`) lives in `CardStackPlugin`, opt-in via
		// a `CardDeck` router marker so the docs TUI keeps its plain-arrow scroll.
		#[cfg(feature = "tui")]
		app.add_message::<bevy::input::keyboard::KeyboardInput>()
			.add_systems(Update, nav_shortcuts);
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
