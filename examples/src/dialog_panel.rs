use bevy::prelude::*;

pub struct DialogPanelPlugin;

impl Plugin for DialogPanelPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, setup_ui)
			.add_systems(
				Update,
				(
					(parse_text_input, button_system),
					(handle_player_message, handle_npc_message),
				)
					.chain(),
			)
			.add_event::<OnPlayerMessage>()
			.add_event::<OnNpcMessage>();
	}
}

#[derive(Event, Deref, DerefMut)]
pub struct OnPlayerMessage(pub String);
#[derive(Event, Deref, DerefMut)]
pub struct OnNpcMessage(pub String);

#[derive(Component)]
pub struct PlayerInput;

#[derive(Component)]
pub struct PlayerOutput;

#[derive(Component)]
pub struct NpcOutput;

fn setup_ui(mut commands: Commands) {
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
				TextBundle::from_section("Player 1: ", TextStyle {
					font_size: 18.,
					..default()
				})
				.with_style(Style { ..default() }),
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
					TextBundle::from_section("I need healing!", TextStyle {
						font_size: 18.,
						..default() // font: (),
						            // color: (),
					})
					.with_style(Style { ..default() }),
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
							TextStyle {
								font_size: 18.,
								..default()
							},
						));
					});
			});
		});
}

fn handle_player_message(
	mut on_player_message: EventReader<OnPlayerMessage>,
	mut player_text: Query<&mut Text, With<PlayerOutput>>,
) {
	for msg in on_player_message.read() {
		for mut text in player_text.iter_mut() {
			text.sections[0].value = format!("Player 1: {}", msg.0);
		}
	}
}

fn handle_npc_message(
	mut on_npc_message: EventReader<OnNpcMessage>,
	mut npc_text: Query<&mut Text, With<NpcOutput>>,
) {
	for msg in on_npc_message.read() {
		for mut text in npc_text.iter_mut() {
			text.sections[0].value = format!("Foxie: {}", msg.0);
		}
	}
}

fn parse_text_input(
	mut evr_char: EventReader<ReceivedCharacter>,
	key_input: Res<ButtonInput<KeyCode>>,
	mut on_submit: EventWriter<OnPlayerMessage>,
	mut query: Query<&mut Text, With<PlayerInput>>,
) {
	for mut text in query.iter_mut() {
		let text = &mut text.sections[0].value;
		if key_input.just_pressed(KeyCode::Enter) {
			on_submit.send(OnPlayerMessage(text.clone()));
			text.clear();
		} else if key_input.just_pressed(KeyCode::Backspace) {
			text.pop();
		} else {
			for ev in evr_char.read() {
				text.push_str(&ev.char);
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
	mut on_submit: EventWriter<OnPlayerMessage>,
	mut input_query: Query<&mut Text, With<PlayerInput>>,
) {
	for (interaction, mut color) in &mut interaction_query {
		match *interaction {
			Interaction::Pressed => {
				*color = PRESSED_BUTTON.into();
				for mut text in input_query.iter_mut() {
					let text = &mut text.sections[0].value;
					on_submit.send(OnPlayerMessage(text.clone()));
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
