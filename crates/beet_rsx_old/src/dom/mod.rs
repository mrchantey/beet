use crate::prelude::*;
#[cfg(target_arch = "wasm32")]
mod beet_dom;
#[cfg(target_arch = "wasm32")]
mod dom_event_registry;
mod event_registry;
#[cfg(target_arch = "wasm32")]
pub use beet_dom::*;
#[cfg(target_arch = "wasm32")]
pub use dom_event_registry::*;
pub use event_registry::*;
mod rs_dom_target;
pub use rs_dom_target::*;
use std::sync::Arc;
use std::sync::Mutex;

#[cfg(target_arch = "wasm32")]
mod browser_dom_target;
#[cfg(target_arch = "wasm32")]
pub use browser_dom_target::*;

// TODO probably shouldnt be a thread local, use once_cell::sync::Lazy
thread_local! {
	#[rustfmt::skip]
	static DOM_TARGET: Arc<Mutex<Box<dyn DomTargetImpl>>> =
		Arc::new(Mutex::new(Box::new(RsDomTarget::new(&().into_node()).unwrap())));
}

/// Mechanism for swapping out:
/// - [`BrowserDomTarget`]
/// - [`RsDomTarget`]
pub struct DomTarget;

impl DomTarget {
	pub fn with<R>(mut func: impl FnMut(&mut dyn DomTargetImpl) -> R) -> R {
		DOM_TARGET.with(|current| {
			let mut current = current.lock().unwrap();
			func(current.as_mut())
		})
	}

	/// Sets the current [`DomTargetImpl`], even if the previous one is poisoned.
	pub fn set(item: impl 'static + Sized + DomTargetImpl) {
		DOM_TARGET.with(|current| {
			*current.lock().unwrap_or_else(|e| e.into_inner()) = Box::new(item);
		});
	}

	pub fn render() -> String {
		DOM_TARGET.with(|current| {
			let current = current.lock().unwrap();
			current.render()
		})
	}

	pub fn update_web_node(
		loc: TreeLocation,
		node: WebNode,
	) -> ParseResult<()> {
		DOM_TARGET.with(|current| {
			let mut current = current.lock().unwrap();
			current.update_web_node(loc, node)
		})
	}
	pub fn update_rsx_attribute(
		loc: TreeLocation,
		key: &str,
		value: &str,
	) -> ParseResult<()> {
		DOM_TARGET.with(|current| {
			let mut current = current.lock().unwrap();
			current.update_rsx_attribute(loc, key, value)
		})
	}
}


pub trait DomTargetImpl {
	/// Mutable in case the impl needs to load the tree location map
	fn tree_location_map(&mut self) -> &TreeLocationMap;
	fn html_constants(&self) -> &HtmlConstants;

	// type Event;
	fn update_web_node(
		&mut self,
		loc: TreeLocation,
		node: WebNode,
	) -> ParseResult<()>;

	fn update_rsx_attribute(
		&mut self,
		loc: TreeLocation,
		key: &str,
		value: &str,
	) -> ParseResult<()>;


	/// just used for testing atm
	fn render(&self) -> String;

	// fn register_event(&self, event: Box<dyn Fn() -> ()>);
}





#[cfg(test)]
#[cfg(all(not(target_arch = "wasm32"), feature = "e2e"))]
mod test {
	use sweet::prelude::*;


	// we immediately click the button,
	// should be before the wasm had a chance to load
	#[sweet::test]
	#[ignore]
	async fn playback_prehydrated() {
		let page =
			visit("http://127.0.0.1:3000/design/interactive/button").await;
		let el = page.find_id("interactive-text").await;
		page.find_id("interactive-button")
			.await
			.click()
			.await
			.unwrap();
		el.as_ref().xpect().to_have_text("value: 0").await;
		expect(&el).to_have_text("value: 1").await;
	}
}
