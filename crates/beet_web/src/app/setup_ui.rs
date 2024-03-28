use super::spawn::DomSimMessage;
use crate::prelude::get_entities_container;
use flume::Sender;
use forky_web::DocumentExt;
use forky_web::HtmlEventListener;
use forky_web::ResizeListener;
use web_sys::Document;
use web_sys::Event;
use web_sys::HtmlButtonElement;



pub fn setup_ui(send: Sender<DomSimMessage>) {
	message_button(
		send.clone(),
		"#create-bee",
		DomSimMessage::SpawnBeeFromFirstNode,
	);
	message_button(send.clone(), "#create-flower", DomSimMessage::SpawnFlower);
	message_button(send.clone(), "#clear-all", DomSimMessage::DespawnAll);


	ResizeListener::new(&get_entities_container(), move |_e| {
		send.send(DomSimMessage::Resize).ok();
	})
	.forget();
}

fn message_button(
	send: Sender<DomSimMessage>,
	selector: &str,
	message: DomSimMessage,
) {
	let target =
		Document::x_query_selector::<HtmlButtonElement>(selector).unwrap();

	HtmlEventListener::new_with_target(
		"click",
		move |_: Event| {
			send.send(message.clone()).ok();
		},
		target,
	)
	.forget();
}
