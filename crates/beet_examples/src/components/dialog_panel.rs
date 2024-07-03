use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use crate::prelude::*;
use serde::Deserialize;
use serde::Serialize;

pub struct DialogPanelPlugin;

impl Plugin for DialogPanelPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, setup_ui)
			.add_systems(
				Update,
				(
					parse_text_input,
					(handle_player_message, handle_npc_message),
				)
					.chain(),
			)
			.add_event::<OnUserMessage>()
			.add_event::<OnNpcMessage>();

		// #[cfg(not(target_arch = "wasm32"))]
		app.add_systems(Update, button_system);
	}
}

#[derive(Event, Deref, DerefMut, Serialize, Deserialize)]
pub struct OnNpcMessage(pub String);

#[derive(Component)]
pub struct PlayerInput;

#[derive(Component)]
pub struct PlayerOutput;

#[derive(Component)]
pub struct StatusOutput;

#[derive(Component)]
pub struct NpcOutput;

fn setup_ui(mut commands: Commands) {
	let text_style = TextStyle {
		font_size: 20.,
		..default()
	};

	commands
		.spawn(NodeBundle {
			style: Style {
				width: Val::Px(300.),
				height: Val::Percent(100.),
				padding: UiRect::all(Val::Px(10.)),
				display: Display::Flex,
				// justify_content: JustifyContent::SpaceBetween,
				flex_direction: FlexDirection::Column,
				..default()
			},
			..default()
		})
		.with_children(|root| {
			// fox output
			root.spawn((
				StatusOutput,
				TextBundle::from_sections([
					TextSection::new("Status: ", text_style.clone()),
					TextSection::new("Loading", text_style.clone()),
				]),
			));
			// root.spawn((
			// 	NpcOutput,
			// 	TextBundle::from_section("Foxie: ", TextStyle {
			// 		font_size: 18.,
			// 		..default()
			// 	})
			// 	.with_style(Style { ..default() }),
			// ));
			// player output
			root.spawn((
				PlayerOutput,
				TextBundle::from_sections([
					TextSection::new("Message: ", text_style.clone()),
					TextSection::new("", text_style.clone()),
				]),
			));

			// player input
			#[cfg(not(target_arch = "wasm32"))]
			root.spawn(NodeBundle {
				style: Style {
					width: Val::Percent(100.),
					height: Val::Px(40.),
					display: Display::Flex,
					flex_direction: FlexDirection::Row,
					justify_content: JustifyContent::SpaceBetween,
					align_items: AlignItems::Center,
					..default()
				},
				background_color: Color::srgba(1., 1., 1., 0.2).into(),
				..default()
			})
			.with_children(|input_area| {
				input_area.spawn((
					TextBundle::from_section(
						"I need healing!",
						text_style.clone(),
					),
					PlayerInput,
				));

				input_area
					.spawn((ButtonBundle {
						style: Style {
							width: Val::Percent(20.),
							height: Val::Percent(100.),
							display: Display::Flex,
							justify_content: JustifyContent::Center,
							align_items: AlignItems::Center,
							..default()
						},
						image: UiImage::default().with_color(NORMAL_BUTTON),
						..default()
					},))
					.with_children(|parent| {
						parent.spawn(TextBundle::from_section(
							// "Submit",
							"Enter",
							text_style.clone(),
						));
					});
			});
		});
}

fn handle_player_message(
	mut on_player_message: EventReader<OnUserMessage>,
	mut player_text: Query<&mut Text, With<PlayerOutput>>,
) {
	for msg in on_player_message.read() {
		for mut text in player_text.iter_mut() {
			text.sections[1].value = msg.0.clone();
		}
	}
}

fn handle_npc_message(
	mut on_npc_message: EventReader<OnNpcMessage>,
	mut npc_text: Query<&mut Text, With<NpcOutput>>,
) {
	for msg in on_npc_message.read() {
		for mut text in npc_text.iter_mut() {
			text.sections[1].value = format!("Foxie: {}", msg.0);
		}
	}
}

fn parse_text_input(
	mut evr_char: EventReader<KeyboardInput>,
	mut on_submit: EventWriter<OnUserMessage>,
	mut query: Query<&mut Text, With<PlayerInput>>,
) {
	for ev in evr_char.read() {
		for mut text in query.iter_mut() {
			let text = &mut text.sections[0].value;
			match &ev.logical_key {
				Key::Enter => {
					on_submit.send(OnUserMessage(text.clone()));
					text.clear();
				}
				Key::Backspace => {
					text.pop();
				}
				Key::Character(char) => {
					text.push_str(char);
				}
				_ => {}
			}
		}
	}
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn button_system(
	mut interaction_query: Query<
		(&Interaction, &mut BackgroundColor),
		(Changed<Interaction>, With<Button>),
	>,
	mut on_submit: EventWriter<OnUserMessage>,
	mut input_query: Query<&mut Text, With<PlayerInput>>,
) {
	for (interaction, mut color) in &mut interaction_query {
		match *interaction {
			Interaction::Pressed => {
				*color = PRESSED_BUTTON.into();
				for mut text in input_query.iter_mut() {
					let text = &mut text.sections[0].value;
					on_submit.send(OnUserMessage(text.clone()));
					text.clear();
				}
			}
			Interaction::Hovered => {
				*color = HOVERED_BUTTON.into();
			}
			Interaction::None => {
				*color = NORMAL_BUTTON.into();
			}
		}
	}
}
