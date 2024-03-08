use forky_web::DocumentExt;
use web_sys::Document;

pub fn setup_dom() { append_style_to_head(); }

// fn run_post_message(relay: Relay) {
// let current_frame = IdIncr::new();
// let mut post_message =
// 	PostMessageRelay::new_with_current_window(relay.clone());
// let _frame = AnimationFrame::new(move || {
// 	post_message.send_all().ok_or(|e| log::error!("{e}"));
// });
// }

fn append_style_to_head() {
	let style_content = include_str!("style.css");
	let style_element = Document::x_create_element("style");
	style_element.set_inner_html(style_content);
	Document::x_head().append_child(&style_element).unwrap();

	let root = Document::x_create_div();
	root.set_inner_html(include_str!("body.html"));
	Document::x_body().append_child(&root).unwrap();
	// Document::x_body().set_inner_html(include_str!("body.html"));
}
