use crate::prelude::*;
use beet_common::prelude::*;
use beet_router::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

/// Helper for common route mapping
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Component)]
pub struct ModifyFileRouteTokens {
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

impl Default for ModifyFileRouteTokens {
	fn default() -> Self {
		Self {
			base_route: None,
			replace_route: vec![],
		}
	}
}


impl ModifyFileRouteTokens {
	pub fn base_route(mut self, base_route: impl Into<PathBuf>) -> Self {
		self.base_route = Some(RoutePath::new(base_route));
		self
	}
	pub fn replace_route<S: ToString>(
		mut self,
		replace: impl IntoIterator<Item = (S, S)>,
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
		(&mut FileRouteTokensSend, &ModifyFileRouteTokens),
		Added<FileRouteTokensSend>,
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
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let entity = world
			.spawn((
				FileRouteTokens::simple_with_func(
					file!(),
					syn::parse_quote!(
						fn get() {}
					),
				)
				.sendit(),
				ModifyFileRouteTokens::default()
					.base_route("/design")
					.replace_route([(
						"crates/beet_build/src/build_codegen/",
						"",
					)]),
			))
			.id();
		world.run_system_once(modify_file_route_tokens).unwrap();
		world
			.get::<FileRouteTokensSend>(entity)
			.unwrap()
			.route_info
			.path
			.to_string()
			.xpect()
			.to_be("/design/modify_file_route_tokens".to_string());
	}
}
