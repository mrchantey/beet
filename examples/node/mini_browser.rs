//! An ittie bittie web browser demonstrating the parsing and rendering capabilities of beet.
//!
//! This demo parses html and markdown only, SPAs and css/js heavy sites need not apply
//!
//! ```sh
//! cargo run --example mini_browser --features _mini_browser -- https://wikipedia.org
//! ```
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((TuiPlugin::default(), AsyncPlugin::default()))
		.add_systems(
			PreUpdate,
			(url_bar_input, history_input, update_url_bar_on_navigate),
		)
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	let args = CliArgs::parse_env();
	let url = args
		.path
		.first()
		.cloned()
		.unwrap_or_else(|| "http://example.com".to_string());

	let navigator = commands.spawn(Navigator::new(url.clone())).id();

	commands.spawn((Layout::vertical(), children![
		TuiTextBox::new("url", &url),
		(
			// listens for responses delivered by Navigator
			RenderedBy(navigator),
			// parses the RenderMedia observer into the entity
			MediaParser::default(),
			// renders this entity on a NodeParsed event,
			// triggering a Changed<TuiWidget> which results in
			// a ratatui refresh
			TuiNodeRenderer::default(),
		)
	]));
}


use beet::exports::bevy_ratatui::event::KeyMessage;
use beet::exports::bevy_ratatui::event::MouseMessage;
use beet::exports::ratatui::crossterm::event::KeyCode;
use beet::exports::ratatui::crossterm::event::KeyModifiers;
use beet::exports::ratatui::crossterm::event::MouseButton;
use beet::exports::ratatui::crossterm::event::MouseEventKind;

/// Handle character input, backspace and enter key in the URL bar.
fn url_bar_input(
	mut commands: Commands,
	mut key_messages: MessageReader<KeyMessage>,
	mut textbox: Query<(&mut TuiWidget, &mut TuiTextBox)>,
	navigators: Query<Entity, With<Navigator>>,
) -> Result {
	let (mut widget, mut textbox) = textbox.single_mut()?;

	for message in key_messages.read().filter(|msg| msg.is_press()) {
		match message.code {
			KeyCode::Enter => {
				let url = Url::parse(&textbox.value);
				commands
					.entity(navigators.single()?)
					.queue_async(|entity| Navigator::navigate_to(entity, url))
			}
			KeyCode::Backspace => {
				textbox.value.pop();
				widget.set_changed();
			}
			KeyCode::Char(char) => {
				textbox.value.push(char);
				widget.set_changed();
			}
			_ => {}
		}
	}
	Ok(())
}

/// Handle browser back/forward navigation via:
///
/// - `Alt+Left` / `Alt+Right` — standard terminal binding, also fired by
///   most mice that report side-buttons (button 4 / button 5) through the
///   terminal's mouse protocol.
/// - `[` / `]` — keyboard shortcuts.
/// - Middle-click fires back as a convenience (uncommon but handy in testing).
fn history_input(
	mut commands: Commands,
	mut key_messages: MessageReader<KeyMessage>,
	mut mouse_messages: MessageReader<MouseMessage>,
	navigators: Query<Entity, With<Navigator>>,
) -> Result {
	let Ok(navigator) = navigators.single() else {
		return Ok(());
	};

	let mut go_back = false;
	let mut go_forward = false;

	for message in key_messages.read().filter(|msg| msg.is_press()) {
		match message.code {
			// Alt+Left — back (also triggered by mouse button 4 in most terminals)
			KeyCode::Left if message.modifiers.contains(KeyModifiers::ALT) => {
				go_back = true;
			}
			// Alt+Right — forward (also triggered by mouse button 5 in most terminals)
			KeyCode::Right if message.modifiers.contains(KeyModifiers::ALT) => {
				go_forward = true;
			}
			// Keyboard shortcuts
			KeyCode::Char('[') => go_back = true,
			KeyCode::Char(']') => go_forward = true,
			_ => {}
		}
	}

	// Some terminals / mouse drivers report side-buttons as middle-button
	// variants; handle ScrollLeft/ScrollRight which kitty and some others emit.
	for message in mouse_messages.read() {
		match message.0.kind {
			MouseEventKind::Down(MouseButton::Middle) => go_back = true,
			MouseEventKind::ScrollLeft => go_back = true,
			MouseEventKind::ScrollRight => go_forward = true,
			_ => {}
		}
	}

	if go_back {
		commands.entity(navigator).queue_async(Navigator::back);
	} else if go_forward {
		commands.entity(navigator).queue_async(Navigator::forward);
	}

	Ok(())
}

/// Propagate URL changes (eg link clicks) back to the URL bar.
fn update_url_bar_on_navigate(
	mut textbox: Query<(&mut TuiWidget, &mut TuiTextBox)>,
	navigators: Populated<&Navigator, Changed<Navigator>>,
) {
	for navigator in navigators.iter() {
		for (mut widget, mut textbox) in textbox.iter_mut() {
			textbox.value = navigator.current_url().to_string();
			if navigator.is_loading() {
				textbox.value.push_str(" (loading...)");
			}
			widget.set_changed();
		}
	}
}



//*
//
// # 🌐 Classic Web-1.0 Sites That Work Great in TUI Browsers
//
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
