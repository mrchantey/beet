use beet::prelude::*;
use bevy::prelude::*;

#[tokio::main]
async fn main() {
	// 1. Add plugins
	BevyRuntime::with_mut(|app| {
		app.add_plugins((
			DefaultPlugins,
			BevyEventRegistry,
			BevyTemplateReloader::new(std::file!()),
		));
	});

	// 2. Define scene
	let scene = rsx! {
		<cam Camera2d/>
		<Counter initial=7/>
	};

	// 3. Spawn scene
	let _entities = RsxToBevy::spawn(scene).unwrap();

	// 4. Run app
	let mut app = BevyRuntime::take();
	app.run();
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
				set(val + 1);
			}>
				"The value is cdertaly "{get}
			</entity>
		}
	}
}
