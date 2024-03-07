// use anyhow::Result;
// use beet::prelude::*;
// use forky_web::DocumentExt;
// use forky_web::HtmlEventListener;
// use web_sys::Document;
// use web_sys::Event;
// use web_sys::HtmlButtonElement;
// use web_sys::HtmlDivElement;


// #[must_use]
// fn setup_ui(
// 	relay: &mut Relay,
// 	container: &HtmlDivElement,
// ) -> Result<HtmlEventListener<Event>> {
// 	let play_button = play_button(&container);
// 	let play_listener = spawn_entity_requester(relay, play_button.clone())?;
// 	Ok(play_listener)
// }


// #[must_use]
// fn spawn_entity_requester(
// 	relay: &mut Relay,
// 	button: HtmlButtonElement,
// ) -> Result<HtmlEventListener<Event>> {
// 	let tx = SpawnEntityHandler::requester(relay);

// 	let event_listener = HtmlEventListener::new_with_target(
// 		"click",
// 		move |_| {
// 			// let obj = AsciiObject {
// 			// 	text: "🐉".to_string(),
// 			// 	position: Vec2::new(0.0, 0.0),
// 			// };
// 			// let tx = tx.clone();
// 			// spawn_local(async move {
// 			// 	tx.clone().request(&obj).await.ok_or(|e| log::error!("{e}"));
// 			// });
// 		},
// 		button.into(),
// 	);

// 	Ok(event_listener)
// }



// fn play_button(container: &HtmlDivElement) -> HtmlButtonElement {
// 	let button = Document::x_create_button();
// 	button.set_class_name("play");
// 	button.set_inner_text("▶");
// 	container.append_child(&button).unwrap();
// 	// Document::x_append_child(&button);
// 	button
// }
