//! # Hello ML Chat
//! Like 'hello ml' but the user enters the prompt.
use beet::examples::scenes;
use beet::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
pub fn main() {
	App::new()
		.add_plugins((
			running_beet_example_plugin, 
			plugin_ml
		))
		.add_systems(
			Startup,
			(
				scenes::camera_2d,
				scenes::ui_terminal_input,
				setup,
			),
		)

		.run();
}

#[rustfmt::skip]
fn setup(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut ev: MessageWriter<OnLogMessage>,
) {
	ev.write(
		OnLogMessage::new("Agent: I can heal or attack, what should i do?",
		OnLogMessage::GAME_COLOR)
		.and_log()
	);

	let bert = asset_server.load::<Bert>("ml/default-bert.ron");
	commands
		.spawn((
			Name::new("Hello ML"),
			HandleWrapper(bert),
			RunWithUserSentence::default(),
			NearestSentence::new(),
		))
		.with_child((
			Name::new("Heal Behavior"), 
			Sentence::new("heal")
		))
		.with_child((
			Name::new("Attack Behavior"), 
			Sentence::new("attack")
		));}
