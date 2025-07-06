use crate::prelude::*;
use axum::extract::FromRequestParts;
use beet_net::prelude::*;

pub struct BundleLayerPlugin<L, R> {
	pub layer: L,
	/// The [`RouterPlugin`] that this is layered on top of.
	pub router_plugin: R,
}

impl<L, R> BundleLayerPlugin<L, R> {
	pub fn new(layer: L, router_plugin: R) -> Self {
		Self {
			layer,
			router_plugin,
		}
	}
}


impl<L, R> RouterPlugin for BundleLayerPlugin<L, R>
where
	R: RouterPlugin,
	L: BundleLayerHandler<State = R::State, Meta = R::Meta>,
	// LayerExtractors: 'static + Send + Sync + FromRequestParts<R::State>,
{
	type State = R::State;
	type Meta = R::Meta;

	fn is_static(&self) -> bool { self.layer.is_static() }
	fn routes(&self) -> Vec<RouteInfo> { self.router_plugin.routes() }
	fn meta(&self) -> Vec<Self::Meta> { self.router_plugin.meta() }

	fn add_routes(&self, router: Router<Self::State>) -> Router<Self::State> {
		self.add_routes_with(router, self)
	}

	fn add_routes_with(
		&self,
		router: Router<Self::State>,
		plugin: &impl RouterPlugin<State = Self::State, Meta = Self::Meta>,
	) -> Router<Self::State> {
		self.router_plugin.add_routes_with(router, plugin)
	}

	fn add_bundle_route<M, H>(
		&self,
		router: Router<Self::State>,
		route_info: RouteInfo,
		handler: H,
		meta: Self::Meta,
	) -> Router<Self::State>
	where
		H: BundleRoute<M, State = Self::State>,
		H::Extractors: 'static + Send + Sync + FromRequestParts<Self::State>,
	{
		self.router_plugin.add_route(
			router,
			route_info,
			BundleLayer::new(self.layer.clone(), handler, meta),
		)
	}
}
