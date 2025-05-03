use super::*;
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::Event;
use web_sys::KeyboardEvent;
use web_sys::MouseEvent;


// TODO this can probably be done with promise.resolve instead of looping
pub async fn await_document_interaction() {
	let done_flag = Rc::new(RefCell::new(false));

	let done_flag_rc = done_flag.clone();
	let _on_click =
		HtmlEventListener::new("mousedown", move |_: MouseEvent| {
			*done_flag_rc.borrow_mut() = true;
		});
	let done_flag_rc = done_flag.clone();
	let _on_scroll = HtmlEventListener::new("scroll", move |_: Event| {
		*done_flag_rc.clone().borrow_mut() = true;
	});
	let done_flag_rc = done_flag.clone();
	let _on_key = HtmlEventListener::new("keydown", move |_: KeyboardEvent| {
		*done_flag_rc.clone().borrow_mut() = true;
	});


	while !*done_flag.borrow() {
		wait_for_16_millis().await;
	}
}
