use axum::Router;
use beet_net::prelude::RouteInfo;

use crate::app::BundleRoute;


pub trait RouterPlugin {
	type State;
	type Meta;
	fn build(self, router: Router<Self::State>) -> Router<Self::State>;

	fn add_route<M>(
		&self,
		router: Router<Self::State>,
		handler: impl IntoBeetRoute<M, State = Self::State>,
	) -> Router<Self::State> {
		handler.into_beet_route(router)
	}
}

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
