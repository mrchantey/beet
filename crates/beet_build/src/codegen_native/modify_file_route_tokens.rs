use crate::prelude::*;
use beet_common::prelude::*;
use beet_router::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

/// Helper for common route mapping
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Component)]
pub struct ModifyRouteFileMethod {
	/// A base path to prepend to the route path
	pub base_route: Option<RoutePath>,
	/// List of strings to replace in the route path
	#[serde(default)]
	pub replace_route: Vec<ReplaceRoute>,
}
/// Replace some part of the route path with another string
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplaceRoute {
	/// The string to replace
	from: String,
	/// The string to replace with
	to: String,
}

impl Default for ModifyRouteFileMethod {
	fn default() -> Self {
		Self {
			base_route: None,
			replace_route: vec![],
		}
	}
}


impl ModifyRouteFileMethod {
	pub fn base_route(mut self, base_route: impl Into<PathBuf>) -> Self {
		self.base_route = Some(RoutePath::new(base_route));
		self
	}
	pub fn replace_route<S1: ToString, S2: ToString>(
		mut self,
		replace: impl IntoIterator<Item = (S1, S2)>,
	) -> Self {
		self.replace_route = replace
			.into_iter()
			.map(|(a, b)| ReplaceRoute {
				from: a.to_string(),
				to: b.to_string(),
			})
			.collect();
		self
	}
}

pub fn modify_file_route_tokens(
	_: TempNonSendMarker,
	mut query: Populated<
		(&mut RouteFileMethod, &ModifyRouteFileMethod),
		Added<RouteFileMethod>,
	>,
) {
	for (mut route, modifier) in query.iter_mut() {
		let mut route_path = if let Some(base_route) = &modifier.base_route {
			base_route
				.join(&route.route_info.path)
				.to_string_lossy()
				.to_string()
		} else {
			route.route_info.path.to_string_lossy().to_string()
		};
		for ReplaceRoute { from, to } in &modifier.replace_route {
			route_path = route_path.replace(from, to);
		}
		route.route_info.path = RoutePath::new(route_path);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::dir;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let entity = world
			.spawn((
				RouteFileMethod::new(&*file!().replace(".rs", "")),
				ModifyRouteFileMethod::default()
					.base_route("/design")
					.replace_route([(
						&format!("/{}", dir!().display()),
						// "crates/beet_build/src/codegen_native/",
						"",
					)]),
			))
			.id();
		world.run_system_once(modify_file_route_tokens).unwrap();
		world
			.get::<RouteFileMethod>(entity)
			.unwrap()
			.route_info
			.path
			.to_string()
			.xpect()
			.to_be("/design/modify_file_route_tokens".to_string());
	}
}
