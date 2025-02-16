//!

use crate::prelude::*;
#[cfg(target_arch = "wasm32")]
mod dom_event_registry;
#[cfg(target_arch = "wasm32")]
mod dom_mounter;
#[cfg(target_arch = "wasm32")]
pub use dom_event_registry::EventRegistry;
#[cfg(target_arch = "wasm32")]
pub use dom_mounter::*;
mod html_node_hydrator;
#[cfg(not(target_arch = "wasm32"))]
mod native_event_registry;
pub use html_node_hydrator::*;
#[cfg(not(target_arch = "wasm32"))]
pub use native_event_registry::EventRegistry;
use std::cell::RefCell;


#[cfg(target_arch = "wasm32")]
mod dom_hydrator;
#[cfg(target_arch = "wasm32")]
pub use dom_hydrator::*;

thread_local! {
	static CURRENT_HYDRATOR: RefCell<Box<dyn DomHydrator>> = RefCell::new(Box::new(HtmlNodeHydrator::new(())));
}
pub struct CurrentHydrator;

impl CurrentHydrator {
	pub fn with<R>(mut func: impl FnMut(&mut dyn DomHydrator) -> R) -> R {
		CURRENT_HYDRATOR.with(|current| {
			let mut current = current.borrow_mut();
			func(current.as_mut())
		})
	}


	pub fn set(item: impl 'static + Sized + DomHydrator) {
		CURRENT_HYDRATOR.with(|current| {
			*current.borrow_mut() = Box::new(item);
		});
	}
}

pub trait DomHydrator {
	fn html_constants(&self) -> &HtmlConstants;

	// type Event;
	fn update_rsx_node(
		&mut self,
		node: RsxNode,
		loc: DomLocation,
	) -> ParseResult<()>;
	/// just used for testing atm
	fn render(&self) -> String;

	// fn register_event(&self, event: Box<dyn Fn() -> ()>);
}
