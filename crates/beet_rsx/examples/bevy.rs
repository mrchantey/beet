use beet_rsx::as_beet::*;
use beet_rsx::prelude::Component;
use bevy::prelude::Node;
use bevy::prelude::*;
use bevy::winit::WinitSettings;

fn main() {
	BevyRuntime::with_mut(|app| {
		app.add_plugins((DefaultPlugins, BevyEventRegistry))
			.insert_resource(WinitSettings::desktop_app());
		// .add_systems(Startup, setup);
	});
	let scene = rsx! {
		<Counter initial=7 />
		<cam Camera2d />
	};
	let _entity = RsxToBevy::spawn(scene).unwrap()[0];
	BevyRuntime::with_mut(|app| {
		app.run();
	});

	// App::new()
	// 	.run();
}


struct Counter {
	initial: i32,
}

impl Component for Counter {
	fn render(self) -> RsxRoot {
		let (get, set) = BevySignal::signal(self.initial);
		let get2 = get.clone();
		rsx! {
			<entity
				runtime:bevy
				Button
				onclick=move |_| {
					let val = get2.clone().get();
					println!("clicked: {}", val);
					set(val + 1);
				}
			>
				"The value is "
				{get}
			</entity>
		}
	}
}


#[allow(unused)]
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	// ui camera
	commands.spawn(Camera2d);
	commands
		.spawn((
			Button,
			Node {
				width: Val::Px(150.0),
				height: Val::Px(65.0),
				border: UiRect::all(Val::Px(5.0)),
				// horizontally center child text
				justify_content: JustifyContent::Center,
				// vertically center child text
				align_items: AlignItems::Center,
				..default()
			},
			BorderColor(Color::BLACK),
			BorderRadius::MAX,
			BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
		))
		.with_child((
			Text::new("Button"),
			// TextFont {
			// 	font: asset_server.load("fonts/FiraSans-Bold.ttf"),
			// 	font_size: 33.0,
			// 	..default()
			// },
			TextColor(Color::srgb(0.9, 0.9, 0.9)),
		));
	// commands
	// 	.spawn(Node {
	// 		width: Val::Percent(100.0),
	// 		height: Val::Percent(100.0),
	// 		align_items: AlignItems::Center,
	// 		justify_content: JustifyContent::Center,
	// 		..default()
	// 	})
	// 	.with_children(|parent| {
	// 		parent
	// 			.spawn((
	// 				Button,
	// 				Node {
	// 					width: Val::Px(150.0),
	// 					height: Val::Px(65.0),
	// 					border: UiRect::all(Val::Px(5.0)),
	// 					// horizontally center child text
	// 					justify_content: JustifyContent::Center,
	// 					// vertically center child text
	// 					align_items: AlignItems::Center,
	// 					..default()
	// 				},
	// 				BorderColor(Color::BLACK),
	// 				BorderRadius::MAX,
	// 				BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
	// 			))
	// 			.with_child((
	// 				Text::new("Button"),
	// 				TextFont {
	// 					font: asset_server.load("fonts/FiraSans-Bold.ttf"),
	// 					font_size: 33.0,
	// 					..default()
	// 				},
	// 				TextColor(Color::srgb(0.9, 0.9, 0.9)),
	// 			));
	// 	});
}
