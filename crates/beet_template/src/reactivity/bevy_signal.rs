use crate::prelude::*;
use beet_common::node::IntoTemplateBundle;
use bevy::prelude::*;
use flume::Receiver;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ReceiveSignalStep;


pub fn signals_plugin(app: &mut App) {
	app.configure_sets(Update, ReceiveSignalStep.after(BindStep))
		.add_systems(Update, receive_text_signals.in_set(ReceiveSignalStep));
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

pub(crate) fn receive_text_signals(
	mut query: Query<(&mut TextSpan, &SignalReceiver<String>)>,
) {
	for (mut text, update) in query.iter_mut() {
		while let Ok(new_text) = update.0.try_recv() {
			text.0 = new_text;
		}
	}
}

impl<T: 'static + Send + Sync + Clone + ToString> IntoTemplateBundle<Self>
	for Getter<T>
{
	fn into_node_bundle(self) -> impl Bundle {
		// changes here should be reflected in maybe_signal.rs
		let string_getter = move || self.get().to_string();
		(
			TextSpan::new(self.get().to_string()),
			SignalReceiver::new(string_getter),
		)
	}
}

#[cfg(test)]
mod test {
	use crate::as_beet::*;
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
	fn signal_nodes() {
		let mut app = App::new();
		app.add_plugins(signals_plugin);
		let (get, set) = signal(5);
		let div = app.world_mut().spawn(rsx! {<div>{get}</div>});
		let text = div.get::<Children>().unwrap()[0];

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
}
