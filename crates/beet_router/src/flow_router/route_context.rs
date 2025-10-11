use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::platform::collections::HashMap;
use std::collections::VecDeque;
use std::ops::ControlFlow;



/// A map stored on the agent of actions in the tree, storing which
/// parts of the path are still yet to be consumed.
/// This allows us to check for exact path matches.
#[derive(Debug, Default, Deref, DerefMut, Component)]
pub struct RouteContextMap(HashMap<Entity, RouteContext>);

impl RouteContextMap {}


#[derive(Debug, Default, Clone)]
pub struct RouteContext {
	/// The non-empty segments of the route path at this point
	/// in the route tree. For the root cx this will be the entire
	/// route. For endpoints to run this must be empty, ie exact path match
	path: VecDeque<String>,
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

	/// False if a [`PathFilter`] was provided and the [`RouteContext::path`]
	/// was not able to consume it completely.
	pub fn parse_filter(&mut self, filter: &PathFilter) -> Result<()> {
		match filter.matches(&mut self.dyn_segments, &mut self.path) {
			ControlFlow::Continue(_) => Ok(()),
			ControlFlow::Break(_) => Err(bevyhow!("PathFilter did not match")),
		}
	}
}
