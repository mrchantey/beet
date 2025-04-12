use crate::prelude::*;
#[cfg(target_arch = "wasm32")]
mod beet_dom;
#[cfg(target_arch = "wasm32")]
mod dom_event_registry;
#[cfg(target_arch = "wasm32")]
pub use beet_dom::*;
#[cfg(target_arch = "wasm32")]
pub use dom_event_registry::*;
#[cfg(not(target_arch = "wasm32"))]
mod native_event_registry;
mod rs_dom_target;
#[cfg(not(target_arch = "wasm32"))]
pub use native_event_registry::*;
pub use rs_dom_target::*;
use std::sync::Arc;
use std::sync::Mutex;


#[cfg(target_arch = "wasm32")]
mod browser_dom_target;
#[cfg(target_arch = "wasm32")]
pub use browser_dom_target::*;

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
}

pub trait DomTargetImpl {
	/// Mutable in case the impl needs to load the tree location map
	fn tree_location_map(&mut self) -> &TreeLocationMap;
	fn html_constants(&self) -> &HtmlConstants;

	// type Event;
	fn update_rsx_node(
		&mut self,
		node: RsxNode,
		loc: TreeLocation,
	) -> ParseResult<()>;

	// TODO update attriute block, update block value


	/// just used for testing atm
	fn render(&self) -> String;

	// fn register_event(&self, event: Box<dyn Fn() -> ()>);
}
