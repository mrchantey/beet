use crate::app::HttpMethodExt;
use axum::Router;
use axum::routing;
use beet_net::prelude::RouteInfo;


/// Common trait for both vanilla axum handlers and handlers that return a bundle.
// This added layer of abstractions means we can avoid the
// complexities of pin box futures etc with handler impls.
pub trait IntoBeetRoute<M> {
	type State;
	fn add_beet_route(
		self,
		router: Router<Self::State>,
		route_info: RouteInfo,
	) -> Router<Self::State>;
}



/// For a `(RouteInfo, H)` tuple where `H` implements axum `Handler<T, S>`
pub struct AxumHandlerIntoBeetRouteMarker;

impl<H, T, S> IntoBeetRoute<(AxumHandlerIntoBeetRouteMarker, T, S)> for H
where
	H: axum::handler::Handler<T, S>,
	T: 'static,
	S: Clone + Send + Sync + 'static,
{
	type State = S;
	fn add_beet_route(
		self,
		router: Router<Self::State>,
		route_info: RouteInfo,
	) -> Router<Self::State> {
		router.route(
			&route_info.path.to_string_lossy(),
			routing::on(route_info.method.into_axum_method(), self),
		)
	}
}
