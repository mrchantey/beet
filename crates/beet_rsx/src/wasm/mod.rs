mod dom_binding;
mod load_client_islands;
pub use dom_binding::*;
pub(crate) use load_client_islands::*;
mod client_only;
mod event_playback;
pub(crate) use client_only::*;
pub(crate) use event_playback::*;


pub fn document_exists() -> bool {
	web_sys::window().map(|w| w.document()).flatten().is_some()
}
