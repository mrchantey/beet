#![cfg(target_arch = "wasm32")]
use crate::prelude::*;
use bevy::prelude::*;
use flume::Receiver;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::window;
use web_sys::MessageEvent;


#[deprecated = "use beet_net instead"]
pub struct PostmessageInputPlugin;

impl Plugin for PostmessageInputPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, postmessage_input)
			.add_systems(Update, postmessage_input_system);
	}
}


#[derive(Resource)]
pub struct PostmessageIn(pub Receiver<String>);

pub fn postmessage_input(mut commands: Commands) {
	let (send, recv) = flume::unbounded();
	commands.insert_resource(PostmessageIn(recv));

	let closure =
		Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
			let value = e.data().as_string().unwrap_or_default();
			if value != "wasm_loaded" {
				send.send(value).unwrap();
			}
		});
	window()
		.unwrap()
		.add_event_listener_with_callback(
			"message",
			closure.as_ref().unchecked_ref(),
		)
		.unwrap();
	closure.forget();
}


pub fn postmessage_input_system(
	postmessage_in: Res<PostmessageIn>,
	mut events: EventWriter<OnUserMessage>,
) {
	for msg in postmessage_in.0.try_iter() {
		events.send(OnUserMessage(msg));
	}
}
