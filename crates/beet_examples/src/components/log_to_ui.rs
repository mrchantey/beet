use beet::prelude::LogOnRun;
use beet::prelude::Running;
use bevy::prelude::*;
use bevy::window::WindowResized;

#[derive(Debug, Default, Component)]
pub struct LogToUi;

fn style() -> TextStyle {
	TextStyle {
		font_size: 32.,
		..Default::default()
	}
}

pub fn log_to_ui(
	mut commands: Commands,
	query: Query<Entity, With<LogToUi>>,
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
	log::info!("\nitems_height: {items_height}\ncontainer_height: {container_height}\nmax_scroll: {max_scroll}");
	return -max_scroll;
}

pub fn scroll_to_bottom_on_resize(
	mut resize_reader: EventReader<WindowResized>,
	parents: Query<&Node>,
	mut list: Query<(&mut Style, &Node, &Parent), With<LogToUi>>,
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
		(With<LogToUi>, Changed<Children>),
	>,
	parents: Query<&Node>,
) {
	for (mut style, node, parent) in list.iter_mut() {
		if let Ok(parent) = parents.get(**parent) {
			style.top = Val::Px(get_top_pos(node, parent));
		}
	}
}

pub fn spawn_log_to_ui(mut commands: Commands) {
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
					LogToUi,
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
