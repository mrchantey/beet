use crate::prelude::*;
use beet_common::node::AttributeLit;
use beet_common::node::IntoTemplateBundle;
use bevy::prelude::*;
use flume::Receiver;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ReceiveSignalStep;


pub fn signals_plugin(app: &mut App) {
	app.configure_sets(Update, ReceiveSignalStep.after(BindStep))
		.add_systems(
			Update,
			(receive_text_node_signals, receive_attribute_value_signals)
				.in_set(ReceiveSignalStep),
		);
}

/// When building without `bevy_default` we assume the target is the web
#[cfg(not(feature = "bevy_default"))]
pub type TextSpan = beet_common::node::TextNode;

/// A component with a [`flume::Receiver`] that can be used to propagate changes
/// throughout the app, for instance in [`receive_text_signals`].
#[derive(Component)]
pub struct SignalReceiver<T>(Receiver<T>);

impl<T: 'static + Send + Sync> SignalReceiver<T> {
	pub fn new(getter: impl 'static + Send + Sync + Fn() -> T) -> Self {
		let (send, recv) = flume::unbounded::<T>();

		effect(move || {
			let value = getter();
			send.send(value).unwrap();
		});

		SignalReceiver(recv)
	}
}

pub(crate) fn receive_text_node_signals(
	mut query: Populated<(&mut TextSpan, &SignalReceiver<String>)>,
) {
	for (mut text, update) in query.iter_mut() {
		while let Ok(new_text) = update.0.try_recv() {
			text.0 = new_text;
		}
	}
}
pub(crate) fn receive_attribute_value_signals(
	mut query: Populated<(&mut AttributeLit, &SignalReceiver<String>)>,
) {
	for (mut lit, update) in query.iter_mut() {
		while let Ok(new_text) = update.0.try_recv() {
			*lit = AttributeLit::new(new_text);
		}
	}
}

impl<T: 'static + Send + Sync + Clone + ToString> IntoTemplateBundle<Self>
	for Getter<T>
{
	fn into_node_bundle(self) -> impl Bundle {
		// changes here should be reflected in maybe_signal.rs
		let get_str = move || self.get().to_string();
		(TextSpan::new(get_str()), SignalReceiver::new(get_str))
	}
	fn into_attribute_bundle(self) -> impl Bundle
	where
		Self: 'static + Send + Sync + Sized,
	{
		let get_str = move || self.get().to_string();
		(AttributeLit::new(get_str()), SignalReceiver::new(get_str))
	}
}

#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn app_signals() {
		let mut app = App::new();
		app.add_plugins(signals_plugin);

		let (get, set) = signal("foo".to_string());

		let entity = app
			.world_mut()
			.spawn((TextSpan::new("foo".to_string()), SignalReceiver::new(get)))
			.id();

		app.world()
			.entity(entity)
			.get::<TextSpan>()
			.unwrap()
			.0
			.xref()
			.xpect()
			.to_be("foo");

		set("bar".to_string());

		app.update();

		app.world()
			.entity(entity)
			.get::<TextSpan>()
			.unwrap()
			.0
			.xref()
			.xpect()
			.to_be("bar");
	}


	#[test]
	fn nodes() {
		let mut app = App::new();
		app.add_plugins(signals_plugin);
		let (get, set) = signal(5);
		let div = app.world_mut().spawn(rsx! {<div>{get}</div>}).id();
		app.world_mut()
			.run_system_once(spawn_templates)
			.unwrap()
			.unwrap();
		let text = app.world().entity(div).get::<Children>().unwrap()[0];

		app.world()
			.entity(text)
			.get::<TextSpan>()
			.unwrap()
			.0
			.xref()
			.xpect()
			.to_be("5");

		set(10);

		app.update();
		app.world()
			.entity(text)
			.get::<TextSpan>()
			.unwrap()
			.0
			.xref()
			.xpect()
			.to_be("10");
	}
	#[test]
	fn attributes() {
		let mut app = App::new();
		app.add_plugins(signals_plugin);
		let (get, set) = signal("foo");
		let div = app.world_mut().spawn(rsx! {<div class={get}/>}).id();
		app.world_mut()
			.run_system_once(spawn_templates)
			.unwrap()
			.unwrap();
		let attr = app.world().entity(div).get::<Attributes>().unwrap()[0];

		app.world()
			.entity(attr)
			.get::<AttributeLit>()
			.unwrap()
			.to_string()
			.xref()
			.xpect()
			.to_be("foo");

		set("bar");

		app.update();
		app.world()
			.entity(attr)
			.get::<AttributeLit>()
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
		app.add_plugins(TemplatePlugin);
		let (get, set) = signal("foo".to_string());
		let template = app.world_mut().spawn(rsx! {<Bar class={get}/>}).id();
		app.update();
		let div = app.world().entity(template).get::<Children>().unwrap()[0];
		let attr = app.world().entity(div).get::<Attributes>().unwrap()[0];

		app.world()
			.entity(attr)
			.get::<AttributeLit>()
			.unwrap()
			.to_string()
			.xref()
			.xpect()
			.to_be("foo");

		set("bar".to_string());

		app.update();
		app.world()
			.entity(attr)
			.get::<AttributeLit>()
			.unwrap()
			.to_string()
			.xref()
			.xpect()
			.to_be("bar");
	}
}
