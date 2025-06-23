use beet_net::prelude::*;
use bevy::prelude::*;


pub struct ClientIslandLoader {
	pub current_url: String,
}

impl ClientIslandLoader {
	#[cfg(not(target_arch = "wasm32"))]
	pub fn new() -> Self {
		Self {
			current_url: "/".to_string(),
		}
	}
	#[cfg(target_arch = "wasm32")]
	pub fn new() -> Self {
		let mut path =
			web_sys::window().unwrap().location().pathname().unwrap();
		if path.len() > 1 && path.ends_with('/') {
			path.pop();
		}
		Self {
			current_url: path.as_str().to_string(),
		}
	}

	/// For a given route, mount the islands if the current URL matches the route.
	pub fn try_mount(
		&self,
		app: &mut App,
		route_info: RouteInfo,
		on_match: impl FnOnce(&mut World),
	) {
		if route_info.path.to_string_lossy() == self.current_url {
			on_match(app.world_mut());
		}
	}
}
