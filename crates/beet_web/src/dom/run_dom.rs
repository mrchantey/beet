use forky_web::DocumentExt;
use web_sys::Document;

pub fn append_html_for_tests() {
	let style_content = include_str!("../../assets/style.css");
	let style_element = Document::x_create_element("style");
	style_element.set_inner_html(style_content);
	Document::x_head().append_child(&style_element).unwrap();

	let root = Document::x_create_div();
	root.set_inner_html(include_str!("body.html"));
	Document::x_body().append_child(&root).unwrap();
	// Document::x_body().set_inner_html(include_str!("body.html"));
}
