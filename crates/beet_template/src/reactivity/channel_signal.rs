use crate::prelude::*;
use bevy::prelude::*;
use flume::Receiver;



pub fn reactivity_plugin(app: &mut App) {
	app.add_systems(Update, update_text);
}



/// Placeholder for [`bevy::prelude::Text`] when building without the `bevy_default` feature.
#[cfg(not(feature = "bevy_default"))]
#[derive(Component)]
pub struct Text(pub String);

#[derive(Component)]
pub struct UpdateText<T>(Receiver<T>);

impl<T: 'static + Send + Sync> UpdateText<T> {
	pub fn new(getter: impl 'static + Send + Sync + Fn() -> T) -> Self {
		let (send, recv) = flume::unbounded::<T>();

		effect(move || {
			let value = getter();
			send.send(value).unwrap();
		});

		UpdateText(recv)
	}
}

fn update_text(mut query: Query<(&mut Text, &UpdateText<String>)>) {
	for (mut text, update) in query.iter_mut() {
		while let Ok(new_text) = update.0.try_recv() {
			text.0 = new_text;
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	/// Example usage
	fn works() {
		let mut app = App::new();
		app.add_plugins(reactivity_plugin);

		let (get, set) = signal("foo".to_string());

		let entity = app
			.world_mut()
			.spawn((Text("foo".to_string()), UpdateText::new(get)))
			.id();

		app.world()
			.entity(entity)
			.get::<Text>()
			.unwrap()
			.0
			.xref()
			.xpect()
			.to_be("foo");

		// ✨ setters are `Clone` and can be called from anywhere
		set("bar".to_string());

		app.update();

		app.world()
			.entity(entity)
			.get::<Text>()
			.unwrap()
			.0
			.xref()
			.xpect()
			.to_be("bar");
	}
}
