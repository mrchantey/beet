//! Navigation utilities for DOM applications.
//!
//! This module provides functions for programmatic navigation
//! in both browser and native environments.

/// Navigates to the specified page path.
///
/// In WASM environments, this uses the browser's location API.
/// In native environments, this is currently unimplemented.
// TODO page member function
#[allow(unused)]
pub fn to_page(path: &str) {
	#[cfg(target_arch = "wasm32")]
	{
		web_sys::window()
			.unwrap()
			.location()
			.set_href(&path)
			.unwrap();
	}
	#[cfg(not(target_arch = "wasm32"))]
	unimplemented!();
}
