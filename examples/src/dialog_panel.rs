use bevy::prelude::*;

pub struct DialogPanelPlugin;

impl Plugin for DialogPanelPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, setup_ui)
			.add_systems(
				Update,
				(
					(parse_text_input, button_system),
					(update_text_input, handle_messages),
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

#[derive(Component, Deref, DerefMut)]
pub struct InputTextField(pub String);
#[derive(Component)]
pub struct MessagesSection;

fn setup_ui(mut commands: Commands) {
	commands
		.spawn(NodeBundle {
			style: Style {
				width: Val::Percent(100.),
				height: Val::Percent(100.),
				justify_content: JustifyContent::SpaceBetween,
				..default()
			},
			..default()
		})
		.with_children(|root| {
			// left
			root.spawn(NodeBundle {
				style: Style { ..default() },
				// background_color: Color::srgba(0., 0., 0., 0.).into(),
				..default()
			});
			// right
			root.spawn((NodeBundle {
				style: Style {
					width: Val::Px(400.),
					// width: Val::Percent((1. - SPLIT_RATIO) * 100.),
					border: UiRect::all(Val::Px(2.)),
					display: Display::Flex,
					flex_direction: FlexDirection::Column,
					..default()
				},
				background_color: Color::linear_rgba(0.1, 0.1, 0.1, 0.1).into(),
				..default()
			},))
				.with_children(|right_col| {
					// message section
					right_col.spawn((
						MessagesSection,
						// ScrollingList::default(),
						NodeBundle {
							style: Style {
								height: Val::Auto,
								// flex_grow: 1.,
								overflow: Overflow::clip_y(),
								// max_height: Val::Percent(100.) - Val::Px(40.),
								display: Display::Flex,
								flex_direction: FlexDirection::Column,
								// width: Val::Percent(100.),
								..default()
							},
							..default()
						},
					));

					// input area
					right_col
						.spawn(NodeBundle {
							style: Style {
								width: Val::Percent(100.),
								height: Val::Px(40.),
								display: Display::Flex,
								flex_direction: FlexDirection::Row,
								..default()
							},
							..default()
						})
						.with_children(|input_area| {
							input_area
								.spawn(NodeBundle {
									style: Style {
										width: Val::Percent(100.),
										height: Val::Percent(100.),
										display: Display::Flex,
										align_items: AlignItems::Center,
										..default()
									},
									background_color: Color::linear_rgb(
										0.2, 0.2, 0.2,
									)
									.into(),
									..default()
								})
								.with_children(|parent| {
									parent.spawn((
										TextBundle::from_section(
											"I need healing!",
											TextStyle {
												font_size: 18.,
												..default() // font: (),
												            // color: (),
											},
										)
										.with_style(Style {
											max_width: Val::Percent(80.),
											// flex_grow: 1.,
											..default()
										})
										.with_style(Style { ..default() }),
										InputTextField(
											"I need healing!".into(),
										),
									));
								});
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
									image: UiImage::default()
										.with_color(NORMAL_BUTTON),
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
		});
}

fn update_text_input(
	mut query: Query<(&mut Text, &InputTextField), Changed<InputTextField>>,
) {
	for (mut text, input) in query.iter_mut() {
		text.sections[0].value = input.0.clone();
	}
}


fn handle_messages(
	mut commands: Commands,
	mut on_player_message: EventReader<OnPlayerMessage>,
	mut on_npc_message: EventReader<OnNpcMessage>,
	message_section: Query<Entity, With<MessagesSection>>,
) {
	for msg in on_player_message.read() {
		commands
			.entity(message_section.iter().next().unwrap())
			.with_children(|parent| {
				parent.spawn(new_message(&msg, true));
			});
	}
	for msg in on_npc_message.read() {
		commands
			.entity(message_section.iter().next().unwrap())
			.with_children(|parent| {
				parent.spawn(new_message(&msg, false));
			});
	}
}


fn new_message(text: &str, is_player: bool) -> impl Bundle {
	let text = if is_player {
		format!("Player 1:\n  {}", text)
	} else {
		format!("Foxie:\n  {}", text)
	};

	TextBundle::from_section(text, TextStyle {
		font_size: 18.,
		..default()
	})
	.with_style(Style {
		width: Val::Percent(80.),
		padding: UiRect::all(Val::Px(10.)),
		..default()
	})
}

fn parse_text_input(
	mut evr_char: EventReader<ReceivedCharacter>,
	key_input: Res<ButtonInput<KeyCode>>,
	mut on_submit: EventWriter<OnPlayerMessage>,
	mut query: Query<&mut InputTextField>,
) {
	for mut field in query.iter_mut() {
		if key_input.just_pressed(KeyCode::Enter) {
			on_submit.send(OnPlayerMessage(field.0.clone()));
			field.clear();
		} else if key_input.just_pressed(KeyCode::Backspace) {
			field.pop();
		} else {
			for ev in evr_char.read() {
				field.push_str(&ev.char);
			}
		}
	}
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn button_system(
	mut interaction_query: Query<
		(&Interaction, &mut BackgroundColor, &Children),
		(Changed<Interaction>, With<Button>),
	>,
	mut on_submit: EventWriter<OnPlayerMessage>,
	mut query: Query<&mut InputTextField>,
	mut text_query: Query<&mut Text>,
) {
	for (interaction, mut color, children) in &mut interaction_query {
		let mut text = text_query.get_mut(children[0]).unwrap();
		match *interaction {
			Interaction::Pressed => {
				log::info!("Pressed");
				text.sections[0].value = "Press".to_string();
				*color = PRESSED_BUTTON.into();
				for mut field in query.iter_mut() {
					on_submit.send(OnPlayerMessage(field.0.clone()));
					field.clear();
				}
			}
			Interaction::Hovered => {
				text.sections[0].value = "Hover".to_string();
				*color = HOVERED_BUTTON.into();
			}
			Interaction::None => {
				text.sections[0].value = "Button".to_string();
				*color = NORMAL_BUTTON.into();
			}
		}
	}
}
