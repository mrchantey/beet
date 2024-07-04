use beet::prelude::LogOnRun;
use beet::prelude::Running;
use bevy::prelude::*;
use bevy::ui::UiSystem;
use bevy::window::WindowResized;


pub struct UiTerminalPlugin;

impl Plugin for UiTerminalPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, ui_terminal).add_systems(
			PostUpdate,
			(scroll_to_bottom_on_resize, scroll_to_bottom_on_append)
				.after(UiSystem::Layout),
		);

		app.register_type::<UiTerminal>();
	}
}



#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct UiTerminal;

fn style() -> TextStyle {
	TextStyle {
		font_size: 32.,
		..Default::default()
	}
}

fn ui_terminal(
	mut commands: Commands,
	query: Query<Entity, With<UiTerminal>>,
	actions: Query<&LogOnRun, Added<Running>>,
) {
	for entity in query.iter() {
		for log in actions.iter() {
			commands.entity(entity).with_children(|parent| {
				parent.spawn(
					// AccessibilityNode(NodeBuilder::new(Role::ListItem)),
					TextBundle::from_section(
						format!("> {}", log.0.clone()),
						style(),
					),
				);
			});
		}
	}
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

pub fn spawn_ui_terminal(mut commands: Commands) {
	commands
		// CONTAINER
		.spawn(NodeBundle {
			style: Style {
				height: Val::Percent(100.),
				width: Val::Percent(100.),
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
		});
}
