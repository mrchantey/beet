use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::path::PathBuf;

/// Helper for common route mapping
#[derive(Debug, Clone, PartialEq, Reflect, Component)]
#[reflect(Component)]
pub struct ModifyRoutePath {
	/// A base path to prepend to the route path
	pub base_route: Option<RoutePath>,
	/// List of strings to replace in the route path
	pub replace_route: Vec<ReplaceRoute>,
}
/// Replace some part of the route path with another string
#[derive(Debug, Clone, PartialEq, Reflect)]
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
	pub fn replace_route(
		mut self,
		from: impl ToString,
		to: impl ToString,
	) -> Self {
		self.replace_route.push(ReplaceRoute {
			from: from.to_string(),
			to: to.to_string(),
		});
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
			base_route.join(&route.path).to_string_lossy().to_string()
		} else {
			route.path.to_string_lossy().to_string()
		};
		for ReplaceRoute { from, to } in &modifier.replace_route {
			route_path = route_path.replace(from, to);
		}
		route.path = RoutePath::new(route_path);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let entity = world
			.spawn((
				RouteFileMethod::new(
					&*file!().replace(".rs", ""),
					HttpMethod::Get,
				),
				ModifyRoutePath::default()
					.base_route("/design")
					.replace_route(
						format!("/{}", dir!().display()),
						// "crates/beet_build/src/codegen_native/",
						"",
					),
			))
			.id();
		world.run_system_cached(modify_route_file_tokens).unwrap();
		world
			.get::<RouteFileMethod>(entity)
			.unwrap()
			.path
			.to_string()
			.xpect_eq("/design/modify_route_file_tokens");
	}
}
