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
		.add_systems(PreUpdate, (url_bar_input, update_url_bar_on_navigate))
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
use beet::exports::ratatui::crossterm::event::KeyCode;

/// handler character input, backspace and enter key
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


// navigation without url bar ie clicking links
// needs to propagate changes to url bar
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
