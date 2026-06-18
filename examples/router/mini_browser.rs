//! A tiny in-terminal web browser shell on the charcell renderer.
//!
//! Parses HTML and markdown only (no SPAs, no heavy css/js). It hosts a
//! [`Navigator`] on the HTTP transport and a live-render host ([`LivePagePlugin`])
//! that paints the navigator's bound page into a persistent
//! [`DoubleBuffer`], with an editable URL bar and back/forward keys.
//!
//! ```sh
//! cargo run --example mini_browser --features _mini_browser -- https://wikipedia.org
//! ```
//!
//! Note: wiring the [`Navigator`]'s async HTTP fetch to render its response into
//! the live host is a follow-up (see `.agents/plans/tui/decisions.md`, Task 10);
//! this shell sets up the host, navigator, and input seam for it.
use beet::prelude::*;
use bevy::input::ButtonState;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyCode;
use bevy::input::keyboard::KeyboardInput;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			// the live terminal host: charcell render + input bridge + repaint.
			CharcellTuiPlugin,
			// link-click navigation + the single-active-page invariant.
			NavigatorPlugin,
			// paint the navigator's bound page into the host DoubleBuffer.
			LivePagePlugin,
			AsyncPlugin::default(),
		))
		.add_systems(PreUpdate, (url_bar_enter, history_keys))
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	let args = CliArgs::parse_env();
	let url = args
		.path
		.first()
		.map(|path| path.to_string())
		.unwrap_or_else(|| "https://example.com".to_string());

	// the live host paints the bound page; the co-located Navigator drives
	// navigation over the HTTP transport (remote URLs). The StdioTerminal shares
	// the host entity so `render_terminal` paints its DoubleBuffer to the real
	// terminal.
	commands.spawn((
		StdioTerminal::default(),
		page_host(terminal_ext::size()),
		Navigator::new(Url::parse(&url)),
	));
	// an editable URL bar bound to the document field `url`.
	commands.spawn((Document::new(val!({ "url": url })), children![
		rsx! { <TextField field={FieldRef::new("url")}/> }
	]));
}

/// Navigate to the URL bar's value on Enter.
fn url_bar_enter(
	mut commands: Commands,
	mut keys: MessageReader<KeyboardInput>,
	bars: Query<&Value, With<Focus>>,
	navigators: Query<Entity, With<Navigator>>,
) -> Result {
	let entered = keys.read().any(|key| {
		key.state == ButtonState::Pressed && key.logical_key == Key::Enter
	});
	if !entered {
		return Ok(());
	}
	if let (Ok(value), Ok(navigator)) = (bars.single(), navigators.single()) {
		let url = Url::parse(value.to_string());
		commands
			.entity(navigator)
			.queue_async(move |entity| Navigator::navigate_to(entity, url));
	}
	Ok(())
}

/// Back/forward via `[` / `]`.
fn history_keys(
	mut commands: Commands,
	mut keys: MessageReader<KeyboardInput>,
	navigators: Query<Entity, With<Navigator>>,
) {
	let Ok(navigator) = navigators.single() else {
		return;
	};
	for key in keys.read().filter(|key| key.state == ButtonState::Pressed) {
		match key.key_code {
			KeyCode::BracketLeft => {
				commands.entity(navigator).queue_async(Navigator::back);
			}
			KeyCode::BracketRight => {
				commands.entity(navigator).queue_async(Navigator::forward);
			}
			_ => {}
		}
	}
}

//*
//
// # 🌐 Classic Web-1.0 Sites That Work Great in TUI Browsers
// # 		These serve as a good starting place for exploring the tui web
// ---
//
// # 📚 Text-heavy knowledge sites
//
// * **HTML Writers Guild Archive**
//   http://www.hwg.org/
//
// * **Internet Archive**
//   https://archive.org/
//
// * **Project Gutenberg**
//   https://www.gutenberg.org/
//
// * **The Jargon File**
//   http://www.catb.org/jargon/html/
//
// * **The Hitchhiker’s Guide to the Galaxy (Earth Edition)**
//   https://www.hhgproject.org/
//
// ---
//
// # 🧠 Hacker / nerd culture sites
//
// * **Eric S. Raymond (catb.org)**
//   http://www.catb.org/
//
// * **The UNIX Heritage Society**
//   https://www.tuhs.org/
//
// * **The TCP/IP Guide**
//   http://www.tcpipguide.com/
//
// * **Linux Documentation Project**
//   https://tldp.org/
//
// ---
//
// # 🎮 Weird / fun old-school sites
//
// * **Zombo.com**
//   http://zombo.com/
//
// * **Dinosaur Comics Archive**
//   https://qwantz.com/
//
// * **textfiles.com**
//   http://textfiles.com/
//
// * **Space Jam (1996 website)**
//   https://www.spacejam.com/1996/
//
// ---
//
// # 🗂 Personal / indie Web-1.0 pages
//
// * **Paul Graham Essays**
//   http://www.paulgraham.com/articles.html
//
// * **ArsDigita Community System Archive**
//   http://philip.greenspun.com/ancient-history/
//
// * **Coding Horror (older posts)**
//   https://blog.codinghorror.com/
//
// ---
//
// # 📜 Sites friendly to text browsers
//
// * **Lynx Browser Project**
//   https://lynx.invisible-island.net/
//
// * **Tildeverse Directory**
//   https://tildeverse.org/
//
// * **SDF Public Access UNIX System**
//   https://sdf.org/
//
// ---
//
// # 🧭 Search engines for the small / classic web
//
// * **Wiby**
//   https://wiby.me/
//
// * **Marginalia Search**
//   https://search.marginalia.nu/
//
// ---
//
// # 💡 Tip
//
// Jump to a random old-school site from your terminal:
//
// ```bash
// w3m https://wiby.me/surprise
// ```
