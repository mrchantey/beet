use crate::prelude::*;
use beet_net::prelude::*;


pub trait RouterPlugin {
	type State;
	type Meta;

	/// Whether routes provided by this plugin are static.
	fn is_static(&self) -> bool;

	/// List of routes that this plugin provides.
	fn routes(&self) -> Vec<RouteInfo>;

	/// List of metadata associated with each route.
	fn meta(&self) -> Vec<Self::Meta>;
	/// Returns a vector of tuples where each tuple contains a `RouteInfo` and its corresponding metadata.
	fn route_metas(&self) -> Vec<(RouteInfo, Self::Meta)> {
		// Combine routes and metas into a vector of tuples
		// where each tuple contains a RouteInfo and its corresponding Meta.
		self.routes()
			.into_iter()
			.zip(self.meta().into_iter())
			.collect()
	}

	fn add_routes(&self, router: Router<Self::State>) -> Router<Self::State>;

	fn add_route<M>(
		&self,
		router: Router<Self::State>,
		handler: impl IntoBeetRoute<M, State = Self::State>,
	) -> Router<Self::State> {
		handler.into_beet_route(router)
	}
}
