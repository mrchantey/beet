use crate::app::BundleRoute;
use crate::app::HttpMethodExt;
use axum::Router;
use axum::routing;
use beet_net::prelude::RouteInfo;



pub trait IntoBeetRoute<M> {
	type State;
	fn into_beet_route(
		self,
		router: Router<Self::State>,
	) -> Router<Self::State>;
}

/// For a `(RouteInfo, F)` tuple where `F` implements `BundleRoute<M>`
pub struct BundleRouteIntoBeetRouteMarker;

impl<F, M> IntoBeetRoute<(BundleRouteIntoBeetRouteMarker, M)> for (RouteInfo, F)
where
	F: BundleRoute<M>,
{
	type State = F::State;
	fn into_beet_route(
		self,
		router: Router<Self::State>,
	) -> Router<Self::State> {
		let route = self.0;
		router.route(
			&route.path.to_string_lossy(),
			self.1.into_method_router(route.method),
		)
	}
}

/// For a `(RouteInfo, H)` tuple where `H` implements axum `Handler<T, S>`
pub struct AxumHandlerIntoBeetRouteMarker;

impl<H, T, S> IntoBeetRoute<(AxumHandlerIntoBeetRouteMarker, T, S)>
	for (RouteInfo, H)
where
	H: axum::handler::Handler<T, S>,
	T: 'static,
	S: Clone + Send + Sync + 'static,
{
	type State = S;
	fn into_beet_route(
		self,
		router: Router<Self::State>,
	) -> Router<Self::State> {
		let route = self.0;
		router.route(
			&route.path.to_string_lossy(),
			routing::on(route.method.into_axum_method(), self.1),
		)
	}
}
