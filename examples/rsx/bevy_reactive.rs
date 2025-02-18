use beet::prelude::*;
use bevy::prelude::*;
use bevy::winit::WinitSettings;

fn main() {
	BevyRuntime::with(|app| {
		app.add_plugins((
			DefaultPlugins,
			BevyEventRegistry,
			BevyTemplateReloader::default(),
		))
		.insert_resource(WinitSettings::desktop_app());
	});
	let scene = rsx! {
		<Counter initial=7/>
		<cam Camera2d/>
	};
	let _root_entity = RsxToBevy::spawn(scene).unwrap()[0];
	BevyRuntime::with(|app| {
		app.run();
	});
}


struct Counter {
	initial: i32,
}

impl beet::prelude::Component for Counter {
	fn render(self) -> RsxRoot {
		let (get, set) = BevySignal::signal(self.initial);
		let get2 = get.clone();
		rsx! {
			<entity runtime:bevy Button onclick=move |_|{
				let val = get2.clone().get();
				set(val + 2);
			}>
				"The value is "{get}
			</entity>
		}
	}
}
