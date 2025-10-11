use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::platform::collections::HashMap;
use std::collections::VecDeque;
use std::ops::ControlFlow;



#[derive(Debug, Component)]
pub struct ExchangeContext {
	/// A map stored on the agent of actions in the tree, storing which
	/// parts of the path are still yet to be consumed.
	/// This allows us to check for exact path matches.
	route_context_map: HashMap<Entity, RouteContext>,
	/// A channel crated by the `flow_route_handler`,
	/// sending this notifies the observer on the [`RouteServer`]
	sender: Sender<Response>,
}

impl ExchangeContext {
	pub fn new(sender: Sender<Response>) -> Self {
		Self {
			route_context_map: default(),
			sender,
		}
	}
	pub fn route_context_map(&self) -> &HashMap<Entity, RouteContext> {
		&self.route_context_map
	}
	pub fn route_context_map_mut(
		&mut self,
	) -> &mut HashMap<Entity, RouteContext> {
		&mut self.route_context_map
	}
	pub(super) fn sender(&self) -> &Sender<Response> { &self.sender }
}

/// A type lazily created for each point in a route tree for each request
/// as its required, ie for matching a [`PathFilter`]
#[derive(Debug, Default, Clone)]
pub struct RouteContext {
	/// The non-empty segments of the route path at this point
	/// in the route tree. For the root cx this will be the entire
	/// route. For endpoints to run this must be empty, ie exact path match
	path: VecDeque<String>,
	/// A hashmap of the dynamic and wildcard parts collected at this
	/// point in the route path. For the route `foo/:bar/*bazz` and the
	/// request `foo/bing/bong/boom?bang=boo` this map will contain:
	/// ```
	/// bar: bing
	/// bazz: bong/boom
	/// ```
	dyn_segments: HashMap<String, String>,
}

impl RouteContext {
	pub fn new(path: impl AsRef<str>) -> Self {
		Self {
			path: path
				.as_ref()
				.split('/')
				.filter(|s| !s.is_empty())
				.map(|s| s.to_string())
				.collect::<VecDeque<_>>(),
			dyn_segments: default(),
		}
	}

	pub fn path(&self) -> &VecDeque<String> { &self.path }

	/// Parse a filter, pulling from the [`Self::path`] as required and storing
	/// any dynamic/wildcard segments in [`Self::dyn_segments`].
	/// Returns [`Err`] if the [`RouteContext::path`]
	/// was not able to match it completely.
	pub fn parse_filter(&mut self, filter: &PathFilter) -> Result<()> {
		match filter.matches(&mut self.dyn_segments, &mut self.path) {
			ControlFlow::Continue(_) => Ok(()),
			ControlFlow::Break(_) => Err(bevyhow!("PathFilter did not match")),
		}
	}
}
