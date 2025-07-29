use std::borrow::Cow;

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use flume::Receiver;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PropagateSignals;

pub struct SignalsPlugin;
impl Plugin for SignalsPlugin {
	fn build(&self, app: &mut App) {
		// BuildTemplates.register_before(app, Update);
		app.init_plugin(schedule_order_plugin).add_systems(
			PropagateSignals,
			(
				receive_string_signals::<String>,
				receive_string_signals::<&'static str>,
				receive_string_signals::<Cow<'static, str>>,
				receive_bool_signals::<bool>,
				receive_num_signals::<f32>,
				receive_num_signals::<f64>,
				receive_num_signals::<u8>,
				receive_num_signals::<u16>,
				receive_num_signals::<u32>,
				// receive_num_signals::<u64>,
				receive_num_signals::<i8>,
				receive_num_signals::<i16>,
				receive_num_signals::<i32>,
				// receive_num_signals::<i64>,
				#[cfg(feature = "bevy_default")]
				propagate_text_signals,
				#[cfg(target_arch = "wasm32")]
				(update_text_nodes, update_attribute_values)
					.chain()
					.run_if(document_exists),
			)
				.chain(),
		);
	}
}
/// Non generic marker to indicate this entity receives signals.
#[derive(Default, Component)]
pub struct ReceivesSignals;
/// A component with a [`flume::Receiver`] that can be used to propagate changes
/// throughout the app, for instance in [`receive_text_signals`].
#[derive(Component)]
#[require(ReceivesSignals)]
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

pub(crate) fn receive_string_signals<
	T: 'static + Send + Sync + Into<String>,
>(
	mut query: Populated<(&mut TextNode, &SignalReceiver<T>)>,
) {
	for (mut lit, update) in query.iter_mut() {
		while let Ok(val) = update.0.try_recv() {
			*lit = TextNode::new(val);
		}
	}
}
pub(crate) fn receive_bool_signals<
	T: 'static + Send + Sync + Clone + Into<bool>,
>(
	mut query: Populated<(&mut TextNode, &mut BoolNode, &SignalReceiver<T>)>,
) {
	for (mut lit, mut num, update) in query.iter_mut() {
		while let Ok(val) = update.0.try_recv() {
			*lit = TextNode::new(val.clone().into().to_string());
			*num = BoolNode::new(val);
		}
	}
}
pub(crate) fn receive_num_signals<
	T: 'static + Send + Sync + Clone + Into<f64>,
>(
	mut query: Populated<(&mut TextNode, &mut NumberNode, &SignalReceiver<T>)>,
) {
	for (mut lit, mut num, update) in query.iter_mut() {
		while let Ok(val) = update.0.try_recv() {
			*lit = TextNode::new(val.clone().into().to_string());
			*num = NumberNode::new(val);
		}
	}
}

/// In bevy_default pass changed TextNode values to TextSpan
#[cfg(feature = "bevy_default")]
fn propagate_text_signals(
	mut query: Populated<(&mut TextSpan, &TextNode), Changed<TextNode>>,
) {
	for (mut span, text) in query.iter_mut() {
		**span = text.0.clone();
	}
}
// TODO we might want to handle Number and Bool types seperately, instead of this
// blanket implementation
impl<T, M> IntoTemplateBundle<(Self, M)> for Getter<T>
where
	T: 'static + Send + Sync + Clone + IntoTemplateBundle<M>,
{
	fn into_template_bundle(self) -> impl Bundle {
		// let get_str = move || self.get().to_string();
		(
			self.get().into_template_bundle(),
			SignalReceiver::new(move || self.get()),
		)
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
		app.add_plugins(SignalsPlugin);

		let (get, set) = signal("foo".to_string());

		let entity = app
			.world_mut()
			.spawn((TextNode::new("foo".to_string()), SignalReceiver::new(get)))
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
		app.world_mut()
			.run_system_once(apply_rsx_snippets)
			.unwrap()
			.unwrap();

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
		app.world_mut()
			.run_system_once(apply_rsx_snippets)
			.unwrap()
			.unwrap();

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
		app.add_plugins(TemplatePlugin)
			.insert_resource(TemplateFlags::None);
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
}
