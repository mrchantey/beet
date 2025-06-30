use crate::prelude::*;
use axum::extract::FromRequestParts;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing;
use beet_net::prelude::*;
use bevy::prelude::*;

/// A form of middleware, accepting a [`BundleRoute`] and wrapping it.
/// This type will be stored in the [`BundleLayer`] alongside the inner
/// [`BundleRoute`] and associated metadata.
pub trait BundleLayerHandler: 'static + Send + Sync + Clone {
	/// The extractors that this layer will use
	type Extractors: 'static + Send + Sync + FromRequestParts<Self::State>;
	type State: 'static + Send + Sync + Clone;
	/// The output type of the layer, which must implement [`IntoResponse`]
	type Output: IntoResponse;
	type Meta: 'static + Send + Sync + Clone;

	/// Specify whether this layer should be included
	/// in ssg output.
	fn is_static(&self) -> bool { true }

	fn handle_bundle_route(
		&self,
		extractors: Self::Extractors,
		bundle: impl Bundle,
		meta: Self::Meta,
	) -> impl Send + Sync + Future<Output = Self::Output>;
}

/// Wraps a [`BundleRoute`] with a [`BundleLayerHandler`].
#[derive(Debug, Clone)]
pub struct BundleLayer<L, R, M> {
	/// The [`BundleLayerHandler`]
	layer: L,
	/// The inner [`BundleRoute`]
	route: R,
	/// The metadata for this [`BundleRoute`]
	meta: M,
}

impl<L, R, M> BundleLayer<L, R, M> {
	pub fn new(layer: L, route: R, meta: M) -> Self {
		Self { layer, route, meta }
	}
}

pub struct BundleLayerIntoBeetRouteMarker;

impl<Layer, Route, Meta, RouteExtractors, LayerExtractors, State, Marker>
	IntoBeetRoute<(
		BundleLayerIntoBeetRouteMarker,
		RouteExtractors,
		LayerExtractors,
		State,
		Marker,
	)> for BundleLayer<Layer, Route, Meta>
where
	RouteExtractors: 'static + Send + Sync + FromRequestParts<State>,
	LayerExtractors: 'static + Send + Sync + FromRequestParts<State>,
	State: 'static + Send + Sync + Clone,
	Layer: BundleLayerHandler<
			Extractors = LayerExtractors,
			State = State,
			Meta = Meta,
		>,
	Meta: 'static + Send + Sync + Clone,
	Route: BundleRoute<Marker, Extractors = RouteExtractors, State = State>,
{
	type State = State;
	fn add_beet_route(
		self,
		router: Router<Self::State>,
		route_info: RouteInfo,
	) -> Router<Self::State> {
		router.route(
			&route_info.path.to_string_lossy(),
			routing::on(
				route_info.method.into_axum_method(),
				async move |layer_extractors,
				            extractors|
				            -> AppResult<Response> {
					let bundle =
						self.route.into_bundle_result(extractors).await?;
					let res = self
						.layer
						.handle_bundle_route(
							layer_extractors,
							bundle,
							self.meta,
						)
						.await
						.into_response();
					Ok(res)
				},
			),
		)
	}
}



#[cfg(test)]
mod test {
	use super::*;
	use axum::extract::Query as QueryParams;
	use axum::response::IntoResponse;
	use axum::response::Response;
	use serde::Deserialize;
	use sweet::prelude::*;

	#[derive(Debug, Clone, Deserialize)]
	struct BundleParams {
		occupation: String,
	}
	#[derive(Debug, Clone, Deserialize)]
	struct LayerParams {
		name: String,
	}

	#[derive(Debug, Clone, Default)]
	struct MyMiddleware;

	impl BundleLayerHandler for MyMiddleware {
		type Extractors = QueryParams<LayerParams>;
		type State = ();
		type Output = Response;
		type Meta = ();

		fn handle_bundle_route(
			&self,
			params: Self::Extractors,
			bundle: impl Bundle,
			_meta: Self::Meta,
		) -> impl Send + Sync + Future<Output = Self::Output> {
			async move {
				BundleResponse::new(rsx! {
					<div>
						<span>name: {params.name.clone()}</span>
						{bundle}
					</div>
				})
				.into_response()
			}
		}
	}

	fn my_bundle_route(params: QueryParams<BundleParams>) -> impl Bundle {
		rsx! {<span>occupation: {params.occupation.clone()}</span>}
	}

	#[sweet::test]
	async fn works() {
		AppRouter::test()
			.add_route("/", BundleLayer::new(MyMiddleware, my_bundle_route,()))
			.render_route(&"/?name=pizzaguy&occupation=delivermepizza".into())
			.await
			.unwrap()
			.xpect()
			.to_be_str("<!DOCTYPE html><html><head></head><body><div><span>name: pizzaguy</span><span>occupation: delivermepizza</span></div></body></html>");
	}
}
