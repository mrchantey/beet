use forky_web::DocumentExt;
use web_sys::Document;
use web_sys::HtmlDivElement;

pub fn setup_dom() {
	append_style_to_head();
	create_container();
}

// fn run_post_message(relay: Relay) {
// let current_frame = IdIncr::new();
// let mut post_message =
// 	PostMessageRelay::new_with_current_window(relay.clone());
// let _frame = AnimationFrame::new(move || {
// 	post_message.send_all().ok_or(|e| log::error!("{e}"));
// });
// }


fn create_container() -> HtmlDivElement {
	let container = Document::x_create_div();
	container.set_class_name("container");
	Document::x_append_child(&container);
	container
}


fn append_style_to_head() {
	let style_content = include_str!("style.css");
	let style_element = Document::x_create_element("style");
	style_element.set_inner_html(style_content);
	Document::x_head().append_child(&style_element).unwrap();
}
