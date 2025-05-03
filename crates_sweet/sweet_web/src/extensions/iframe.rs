use crate::*;
use extend::ext;
use wasm_bindgen::JsCast;
use web_sys::HtmlIFrameElement;

#[ext]
pub impl HtmlIFrameElement {
	fn x_reload(&self) {
		self.content_window().unwrap().location().reload().unwrap();
	}

	async fn x_set_source_async(&self, url: &str) {
		self.set_src(&url);
		self.x_wait_for_load().await;
		if self.content_document().is_none() {
			panic!("tried to load url: {url}\niframe content document is null, if you can see the page this is likely a cors issue");
		}
	}

	async fn x_reload_async(&self) {
		self.x_reload();
		let this = self.clone().unchecked_into();
		HtmlEventListener::wait_with_target("load", this).await;
	}

	async fn x_wait_for_load(&self) {
		let this = self.clone().unchecked_into();
		HtmlEventListener::wait_with_target("load", this).await;
	}
}

// seems like it works without this, same origin = same thread?
// async fn x_reload_async_while_listening(&self){
// let window = self.content_window().unwrap();
// HtmlEventListener::wait_with_target_and_while_listening(
// 	"load",
// 	this,
// 	move || {
// 		let location = window.location();
// 		location.reload().unwrap();
// 	},
// )
// .await;
// }
