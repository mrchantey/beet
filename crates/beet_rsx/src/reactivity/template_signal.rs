use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemId;
use bevy::prelude::*;
use flume::Receiver;
use flume::Sender;

// creates an OnSpawn that performs the following:
// 1. Add a [`TextNode`] with the initial getter value
// 2. Create an [`effect`] to send a [`DirtySignals`] and queue [`ReactiveApp`]update
// 3. Add an [`Effect`] system to update the text node on change.
impl<T> IntoBundle<Self> for Getter<T>
where
	T: 'static + Send + Sync + Clone + ToString,
{
	fn into_bundle(self) -> impl Bundle {
		OnSpawn::new(move |entity| {
			entity.insert(TextNode::new(self.clone()().to_string()));
			let id = entity.id();
			let sender = entity
				.world_scope(|world| world.resource::<DirtySignals>().sender());
			let func = self.clone();

			// create an effect that will run whenever func is updated.
			// in web this will RequestAnimationFrame, we may need an
			// equivelent for native.
			effect(move || {
				// subscribe to changes
				let _ = func.clone()();
				// ignore errors if receiver dropped
				sender.send(id).ok();
				ReactiveApp::queue_update();
				// request animation frame
			});
			let system_id = entity.world_scope(move |world| {
				world.register_system(move |mut query: Query<&mut TextNode>| {
					if let Ok(mut text) = query.get_mut(id) {
						text.0 = self.clone()().to_string();
					} else {
						// warn?
						warn!(
							"Effect expected an entity with a Text node, none found"
						);
					}
				})
			});

			entity.insert(Effect(system_id));
		})
	}
}

/// An mpsc channel for signals to emit a 'this entity is dirty' event.
/// In combination with a mechanism like `request_animation_frame` this can
/// be used as a reactivity mechanism.
#[derive(Resource)]
pub struct DirtySignals {
	send: Sender<Entity>,
	recv: Receiver<Entity>,
}

impl Default for DirtySignals {
	fn default() -> Self {
		let (send, recv) = flume::unbounded();
		Self { send, recv }
	}
}

impl DirtySignals {
	pub fn sender(&self) -> Sender<Entity> { self.send.clone() }
}


/// Collects all [`DirtySignals::recv`], then deduplicates and
/// runs each effect.
pub fn flush_signals(
	mut commands: Commands,
	dirty: ResMut<DirtySignals>,
	effects: Query<&Effect>,
) {
	let mut entities = Vec::new();
	while let Ok(entity) = dirty.recv.try_recv() {
		entities.push(entity);
	}
	entities.sort();
	entities.dedup();
	for entity in entities {
		if let Ok(effect) = effects.get(entity) {
			commands.run_system(effect.0);
		}
	}
}

#[derive(Component)]
pub struct Effect(SystemId);


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn app_signals() {
		let mut app = App::new();
		app.add_plugins(SignalsPlugin);

		let (get, set) = signal("foo".to_string());

		let entity = app
			.world_mut()
			.spawn((TextNode::new("foo".to_string()), get.into_bundle()))
			.id();

		app.world()
			.entity(entity)
			.get::<TextNode>()
			.unwrap()
			.0
			.xref()
			.xpect()
			.to_be("foo");

		set("bar".to_string());

		app.update();

		app.world()
			.entity(entity)
			.get::<TextNode>()
			.unwrap()
			.0
			.xref()
			.xpect()
			.to_be("bar");
	}


	#[test]
	fn nodes() {
		let mut app = App::new();
		app.add_plugins(SignalsPlugin);
		let (get, set) = signal(5u32);
		let div = app
			.world_mut()
			.spawn(rsx! {<div>{get}</div>})
			.get::<Children>()
			.unwrap()[0];
		let text = app.world().entity(div).get::<Children>().unwrap()[0];
		app.world_mut().run_schedule(ApplySnippets);

		app.world()
			.entity(text)
			.get::<TextNode>()
			.unwrap()
			.0
			.xref()
			.xpect()
			.to_be("5");

		set(10);

		app.update();
		app.world()
			.entity(text)
			.get::<TextNode>()
			.unwrap()
			.0
			.xref()
			.xpect()
			.to_be("10");
	}
	#[test]
	fn attributes() {
		let mut app = App::new();
		app.add_plugins(SignalsPlugin);
		let (get, set) = signal("foo");
		let div = app
			.world_mut()
			.spawn(rsx! {<div class={get}/>})
			.get::<Children>()
			.unwrap()[0];
		let attr = app.world().entity(div).get::<Attributes>().unwrap()[0];
		app.world_mut().run_schedule(ApplySnippets);

		app.world()
			.entity(attr)
			.get::<TextNode>()
			.unwrap()
			.to_string()
			.xref()
			.xpect()
			.to_be("foo");

		set("bar");

		app.update();
		app.world()
			.entity(attr)
			.get::<TextNode>()
			.unwrap()
			.to_string()
			.xref()
			.xpect()
			.to_be("bar");
	}
	#[test]
	fn attribute_blocks() {
		#[derive(Default, Buildable, AttributeBlock)]
		struct Foo {
			class: Option<MaybeSignal<String>>,
		}

		#[template]
		fn Bar(#[field(flatten)] foo: Foo) -> impl Bundle {
			rsx! { <div {foo}/> }
		}

		let mut app = App::new();
		app.add_plugins(ApplyDirectivesPlugin);
		let (get, set) = signal("foo".to_string());
		let template = app
			.world_mut()
			.spawn(rsx! {<Bar class={get}/>})
			.get::<Children>()
			.unwrap()[0];
		app.update();
		let template_inner =
			app.world().entity(template).get::<Children>().unwrap()[0];
		let div = app
			.world()
			.entity(template_inner)
			.get::<Children>()
			.unwrap()[0];
		let attr = app.world().entity(div).get::<Attributes>().unwrap()[0];

		app.world()
			.entity(attr)
			.get::<TextNode>()
			.unwrap()
			.to_string()
			.xref()
			.xpect()
			.to_be("foo");

		set("bar".to_string());

		app.update();
		app.world()
			.entity(attr)
			.get::<TextNode>()
			.unwrap()
			.to_string()
			.xref()
			.xpect()
			.to_be("bar");
	}
	#[sweet::test]
	async fn reactive_app() {
		use beet_utils::time_ext;

		let mut app = App::new();
		app.add_plugins(SignalsPlugin);
		app.set_runner(ReactiveApp::runner);

		let world = app.world_mut();

		let (get, set) = signal(5);
		let entity = world.spawn(get.into_bundle()).id();
		world
			.entity(entity)
			.get::<TextNode>()
			.unwrap()
			.0
			.clone()
			.xpect()
			.to_be("5");
		app.run();
		set(7);
		// yield for queue_microtask
		time_ext::sleep_secs(0).await;
		ReactiveApp::with(|app| {
			app.world()
				.entity(entity)
				.get::<TextNode>()
				.unwrap()
				.0
				.clone()
				.xpect()
				.to_be("7");
		});
	}
}
