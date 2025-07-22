use crate::prelude::*;
use beet_core::prelude::HierarchyQueryExtExt;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::path::PathBuf;

/// Helper for common route mapping
#[derive(Debug, Clone, PartialEq, Component)]
pub struct ModifyRoutePath {
	/// A base path to prepend to the route path
	pub base_route: Option<RoutePath>,
	/// List of strings to replace in the route path
	pub replace_route: Vec<ReplaceRoute>,
}
/// Replace some part of the route path with another string
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceRoute {
	/// The string to replace
	from: String,
	/// The string to replace with
	to: String,
}

impl Default for ModifyRoutePath {
	fn default() -> Self {
		Self {
			base_route: None,
			replace_route: vec![],
		}
	}
}


impl ModifyRoutePath {
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

pub fn modify_route_file_tokens(
	parents: Query<&ChildOf>,
	modifiers: Query<&ModifyRoutePath>,
	mut query: Populated<
		(Entity, &mut RouteFileMethod),
		Changed<RouteFileMethod>,
	>,
) {
	for (entity, mut route) in query.iter_mut() {
		let Some(modifier) = parents
			.iter_ancestors_inclusive(entity)
			.find_map(|e| modifiers.get(e).ok())
		else {
			continue;
		};

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
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let entity = world
			.spawn((
				RouteFileMethod::new(&*file!().replace(".rs", "")),
				ModifyRoutePath::default()
					.base_route("/design")
					.replace_route([(
						&format!("/{}", dir!().display()),
						// "crates/beet_build/src/codegen_native/",
						"",
					)]),
			))
			.id();
		world.run_system_cached(modify_route_file_tokens).unwrap();
		world
			.get::<RouteFileMethod>(entity)
			.unwrap()
			.route_info
			.path
			.to_string()
			.xpect()
			.to_be("/design/modify_route_file_tokens");
	}
}
