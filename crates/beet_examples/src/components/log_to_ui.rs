use crate::prelude::DoNotSerialize;
use beet::prelude::LogOnRun;
use beet::prelude::Running;
use bevy::prelude::*;

#[derive(Debug, Default, Component)]
pub struct LogToUi;



pub fn log_to_ui(
	actions: Query<&LogOnRun, Added<Running>>,
	mut query: Query<&mut Text, With<LogToUi>>,
) {
	for mut text in query.iter_mut() {
		for log in actions.iter() {
			text.sections[0].value.push_str(&log.0);
			text.sections[0].value.push_str("\n");
		}
	}
}

const TEST: &str = r#"
val 1
val 2
val 3
val 4
val 5
val 6
val 7
val 8
val 9
val 10
val 11
"#;


pub fn spawn_log_to_ui(mut commands: Commands) {
	commands
		.spawn((
			NodeBundle {
				style: Style {
					flex_direction: FlexDirection::ColumnReverse,
					// align_items: AlignItems::Center,
					..default()
				},
				..default()
			},
			// ScrollingList::default(),
			// AccessibilityNode(NodeBuilder::new(Role::List)),
		))
		.with_children(|parent| {
			parent.spawn((
				DoNotSerialize,
				LogToUi,
				TextBundle::from_sections([TextSection::new(
					TEST,
					TextStyle {
						// This font is loaded and will be used instead of the default font.
						// font: asset_server.load("fonts/FiraSans-Bold.ttf"),
						font_size: 60.0,
						..default()
					},
				)]),
			));
		});
}
