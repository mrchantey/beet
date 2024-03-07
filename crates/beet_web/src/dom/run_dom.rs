use anyhow::Result;
use beet::exports::Deserialize;
use beet::exports::Serialize;
use beet::prelude::*;
use bevy_math::prelude::*;
use forky_core::ResultTEExt;
use forky_web::wait_for_16_millis;
use forky_web::DocumentExt;
use wasm_bindgen_futures::spawn_local;
use web_sys::Document;
use web_sys::HtmlDivElement;
use web_sys::HtmlElement;

pub fn run_dom_sync(relay: Relay, graph: BehaviorGraph<CoreNode>) {
	spawn_local(async move {
		run_dom(relay, graph).await.ok_or(|e| log::error!("{e}"));
	});
}

pub async fn run_dom(
	mut relay: Relay,
	graph: BehaviorGraph<CoreNode>,
) -> Result<()> {
	let relay = &mut relay;
	append_style_to_head();
	let container = create_container();

	let mut create_entity = SpawnEntityHandler::requester(relay);
	let mut create_graph_entity =
		SpawnBehaviorEntityHandler::<CoreNode>::requester(relay);

	let _flower_id = create_entity
		.request(
			&SpawnEntityPayload::default()
				.with_position(Vec3::new(-0.5, 0., 0.)),
		)
		.await?;
	let bee_id = create_graph_entity
		.request(&SpawnBehaviorEntityPayload::new(
			graph,
			Some(Vec3::new(0.5, 0., 0.)),
			true,
		))
		.await?;

	let mut bee_position_updated = PositionSender::subscriber(relay, bee_id)?;
	// let mut flower_position_updated =
	// 	PositionSender::subscriber(relay, flower_id)?;


	let bee_el = create_dom_entity(&container, "🐝", Vec2::new(0.5, 0.));
	let _flower_el = create_dom_entity(&container, "🌻", Vec2::new(-0.5, 0.));

	loop {
		if let Ok(pos) = bee_position_updated.try_recv() {
			set_position(&bee_el, pos.xy(), &container);
		}
		// if let Ok(pos) = flower_position_updated.try_recv() {
		// 	set_position(&flower_el, pos.xy(), &container);
		// }

		wait_for_16_millis().await;
	}
}

// fn run_post_message(relay: Relay) {
// let current_frame = IdIncr::new();
// let mut post_message =
// 	PostMessageRelay::new_with_current_window(relay.clone());
// let _frame = AnimationFrame::new(move || {
// 	post_message.send_all().ok_or(|e| log::error!("{e}"));
// });
// }

fn create_dom_entity(
	container: &HtmlDivElement,
	text: &str,
	position: Vec2,
) -> HtmlDivElement {
	let div = Document::x_create_div();
	div.set_inner_text(text);
	div.set_class_name("entity");
	container.append_child(&div).unwrap();
	set_position(&*div, position, container);
	div
}


fn create_container() -> HtmlDivElement {
	let container = Document::x_create_div();
	container.set_class_name("container");
	Document::x_append_child(&container);
	container
}

fn set_position<'a>(
	el: &HtmlElement,
	position: Vec2,
	container: &HtmlDivElement,
) {
	let container_width = container.client_width() as f32;
	let container_height = container.client_height() as f32;
	let child_width = el.client_width() as f32;
	let child_height = el.client_height() as f32;


	let left = (container_width / 2.0) + (position.x * (container_width / 2.0))
		- (child_width / 2.0);
	let top = (container_height / 2.0)
		+ (position.y * (container_height / 2.0))
		- (child_height / 2.0);

	el.set_attribute("style", &format!("left: {}px; top: {}px;", left, top))
		.unwrap();
}

fn append_style_to_head() {
	let style_content = include_str!("style.css");
	let style_element = Document::x_create_element("style");
	style_element.set_inner_html(style_content);
	Document::x_head().append_child(&style_element).unwrap();
}
