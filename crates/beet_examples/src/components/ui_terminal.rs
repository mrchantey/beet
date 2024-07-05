use crate::prelude::OnUserMessage;
use beet::prelude::*;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::ui::UiSystem;
use bevy::window::WindowResized;

pub struct UiTerminalPlugin;

impl Plugin for UiTerminalPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(BeetDebugPlugin::new(append_text))
			.add_systems(Update, (log_log_on_run.pipe(append_text),parse_text_input))
			.add_systems(
				PostUpdate,
				(scroll_to_bottom_on_resize, scroll_to_bottom_on_append)
					.after(UiSystem::Layout),
			)
			.register_type::<UiTerminal>()
			.register_type::<PlayerInput>()
			/*-*/;
	}
}


#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct PlayerInput;


#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct UiTerminal;

fn style() -> TextStyle {
	TextStyle {
		font_size: 32.,
		..Default::default()
	}
}

fn append_text(
	values: In<Vec<String>>,
	mut commands: Commands,
	terminals: Query<Entity, With<UiTerminal>>,
) {
	for entity in terminals.iter() {
		for text in values.iter() {
			commands.entity(entity).with_children(|parent| {
				parent.spawn(
					// AccessibilityNode(NodeBuilder::new(Role::ListItem)),
					TextBundle::from_section(format!("> {text}"), style()),
				);
			});
		}
	}
}

fn log_log_on_run(query: Query<&LogOnRun, Added<Running>>) -> Vec<String> {
	query.iter().map(|log| log.0.to_string()).collect()
}

fn get_top_pos(node: &Node, parent: &Node) -> f32 {
	let items_height = node.size().y;
	let container_height = parent.size().y;
	let max_scroll = (items_height - container_height).max(0.);
	// log::info!("\nitems_height: {items_height}\ncontainer_height: {container_height}\nmax_scroll: {max_scroll}");
	return -max_scroll;
}

fn scroll_to_bottom_on_resize(
	mut resize_reader: EventReader<WindowResized>,
	parents: Query<&Node>,
	mut list: Query<(&mut Style, &Node, &Parent), With<UiTerminal>>,
) {
	for _ev in resize_reader.read() {
		for (mut style, node, parent) in list.iter_mut() {
			if let Ok(parent) = parents.get(**parent) {
				style.top = Val::Px(get_top_pos(node, parent));
			}
		}
	}
}

pub fn scroll_to_bottom_on_append(
	mut list: Query<
		(&mut Style, &Node, &Parent),
		(With<UiTerminal>, Changed<Children>),
	>,
	parents: Query<&Node>,
) {
	for (mut style, node, parent) in list.iter_mut() {
		if let Ok(parent) = parents.get(**parent) {
			style.top = Val::Px(get_top_pos(node, parent));
		}
	}
}

pub fn spawn_ui_terminal_with_input(commands: Commands) {
	spawn_ui_terminal(commands, true);
}
pub fn spawn_ui_terminal_no_input(commands: Commands) {
	spawn_ui_terminal(commands, false);
}


fn spawn_ui_terminal(mut commands: Commands, user_input: bool) {
	commands
		// CONTAINER
		.spawn(NodeBundle {
			style: Style {
				height: Val::Percent(100.),
				width: Val::Percent(100.),
				justify_content: JustifyContent::SpaceBetween,
				// align_self: AlignSelf::Stretch,
				flex_direction: FlexDirection::Column,
				overflow: Overflow::clip(),
				..default()
			},
			// background_color: Color::srgb(0.10, 0.10, 0.10).into(),
			..default()
		})
		.with_children(|parent| {
			parent
				// LIST
				.spawn((
					UiTerminal,
					NodeBundle {
						style: Style {
							padding: UiRect::all(Val::Px(10.)),
							flex_direction: FlexDirection::Column,
							// align_items: AlignItems::Center,
							..default()
						},
						..default()
					},
					// ScrollingList::default(),
					// AccessibilityNode(NodeBuilder::new(Role::List)),
				));
			// ))
			// .with_children(|parent| {
			// 	// SCROLL TEST ITEMS
			// 	for i in 0..30 {
			// 		parent.spawn(
			// 			// AccessibilityNode(NodeBuilder::new(Role::ListItem)),
			// 			TextBundle::from_section(
			// 				format!("Item {i}"),
			// 				style(),
			// 			),
			// 		);
			// 	}
			// });
			if !user_input {
				return;
			}
			parent
				.spawn(NodeBundle {
					style: Style {
						width: Val::Percent(100.),
						height: Val::Px(40.),
						padding: UiRect::all(Val::Px(10.)),
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
						TextBundle::from_section("I need healing!", style()),
						PlayerInput,
					));
				});
		});
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
