use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use flume::Receiver;
use flume::Sender;

// Subscribe to changes in a [`SignalEffect`] and queue a deduplicated app update,
// which will call the effect in [`flush_signals`].
// This is seperate from the Getter IntoBundle impl due to orphan rule
pub fn propagate_signal_effect(world: DeferredWorld, cx: HookContext) {
	let entity = cx.entity;
	let signal = world.entity(entity).get::<SignalEffect>()
		.unwrap(/* must exist */);
	let subscribe = signal.effect_subscriber();

	let sender = world.resource::<DirtySignals>().sender();

	effect(move || {
		subscribe();
		// ignore errors if receiver dropped
		sender.send(entity).ok();
		ReactiveApp::queue_update();
	});
}

/// An mpsc channel for signals to emit a 'this entity is dirty' event,
/// see [`flush_signals`]
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


/// Collects all [`DirtySignals::recv`], then runs each effect deduplicated.
pub fn flush_signals(
	mut commands: Commands,
	dirty: ResMut<DirtySignals>,
	effects: Query<&SignalEffect>,
) {
	let mut entities = Vec::new();
	while let Ok(entity) = dirty.recv.try_recv() {
		if !entities.contains(&entity) {
			entities.push(entity);
		}
	}
	for entity in entities {
		if let Ok(effect) = effects.get(entity) {
			commands.run_system(effect.system_id());
		}
	}
}

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
	fn text_nodes() {
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
			class: Option<DerivedGetter<String>>,
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

	#[test]
	fn bundle_node() {}


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
