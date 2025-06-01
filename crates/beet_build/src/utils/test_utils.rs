// use bevy::app::Plugins;
// use bevy::prelude::*;
// use


// #[extend::ext(name=MyTypeExt)]
// pub impl<T> T
// where
// 	T: Bundle,
// {
// 	fn into_scene<M>(self, plugins: impl Plugins<M>) -> String {
// 		let mut app = App::new();
// 		app.add_plugins(plugins);
// 		app.world_mut().spawn(self);
// 		app.update();
// 		let world = app.world();
// 		let scene = DynamicScene::from_world(world);

// 		let type_registry = world.resource::<AppTypeRegistry>();
// 		let type_registry = type_registry.read();
// 		let scene = scene.serialize(&type_registry).unwrap();
// 		scene
// 	}
// }



// pub fn into_scene<PM>(
// 	plugins: impl Clone + Plugins<PM>,
// 	a: impl Bundle,
// 	// b: impl Bundle,
// ) -> String {
// 	let mut app = App::new();
// 	app.add_plugins(plugins.clone());
// 	app.world_mut().spawn(a);
// 	app.update();
// 	let world = app.world();
// 	let scene = DynamicScene::from_world(world);

// 	let type_registry = world.resource::<AppTypeRegistry>();
// 	let type_registry = type_registry.read();
// 	let scene1 = scene.serialize(&type_registry).unwrap();
// 	scene1

// }
