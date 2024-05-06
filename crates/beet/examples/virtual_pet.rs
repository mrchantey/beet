// use beet::prelude::*;
use bevy::prelude::*;
use example_plugin::ExamplePlugin;
#[path = "common/example_plugin.rs"]
mod example_plugin;

fn main() {
	App::new()
		.add_plugins(ExamplePlugin)
		.add_systems(Startup, (setup, setup_ui))
		.run();
}


fn setup(mut commands: Commands) {
	commands.spawn(Text2dBundle {
		text: Text::from_section(":)", TextStyle { ..default() }),
		..default()
	});
	// .spawn((SequenceSelector::default(), Running))
	// .with_children(|parent| {
	// 	parent.spawn((
	// 		// LogOnRun("Hello".into()),
	// 		InsertOnRun(RunResult::Success),
	// 	));
	// 	parent.spawn((
	// 		// LogOnRun("World".into()),
	// 		InsertOnRun(RunResult::Success),
	// 	));
	// });
}


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
		.with_children(|parent| {
			// left
			parent.spawn(NodeBundle {
				background_color: Color::srgba(0., 0., 0., 0.).into(),
				..default()
			});
			// right
			parent.spawn(NodeBundle {
				style: Style {
					width: Val::Px(400.),
					border: UiRect::all(Val::Px(2.)),
					..default()
				},
				background_color: Color::srgba(0., 0., 1., 0.5).into(),
				..default()
			});
		});
}
