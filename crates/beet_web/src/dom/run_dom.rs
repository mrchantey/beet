use forky_web::DocumentExt;
use web_sys::Document;

pub fn append_html_for_tests() {
	let style_content = include_str!("../../assets/style.css");
	let style_element = Document::x_create_element("style");
	style_element.set_inner_html(style_content);
	Document::x_head().append_child(&style_element).unwrap();

	let root = Document::x_create_div();

	let html = include_str!("../../assets/index.html");
	let start_tag = "<body>";
	let end_tag = "</body>";
	let start_index = html.find(start_tag).unwrap() + start_tag.len();
	let end_index = html.find(end_tag).unwrap();
	let body_content = &html[start_index..end_index];

	root.set_inner_html(body_content);
	Document::x_body().append_child(&root).unwrap();
}
